// crate/engines/execution/engine.rs
use rhai::{Engine, AST, Scope, Dynamic, Map};
use crate::managers::driver::{ActiveDriver, ResourceBundle};
use crate::engines::execution::CompiledDriver;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ExecutionEngine {
    engine: Engine,
}

impl ExecutionEngine {
    pub fn new() -> Self {
        let mut engine = Engine::new();

        // --- Utilitaires ---
        engine.register_fn("print_debug", |msg: &str| {
            println!("\x1b[34m[RHAI DEBUG]\x1b[0m {}", msg);
        });

        engine.register_fn("now", || {
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
        });

        engine.register_fn("parse_json", |json_str: &str| {
            serde_json::from_str::<Map>(json_str).unwrap_or_default()
        });

        Self { engine }
    }

    pub fn prepare(&self, bundle: &ActiveDriver) -> Result<CompiledDriver, String> {
        // Compilation Gateway (cherche main.rhai dans la map des scripts chargés par TemplateManager)
        let gateway_content = bundle.gateway.scripts.get("main.rhai")
        .ok_or_else(|| format!("Gateway '{}': main.rhai not found", bundle.gateway_id))?;

        let gateway_ast = self.engine.compile(gateway_content)
        .map_err(|e| format!("Error compiling Gateway ({}): {}", bundle.gateway_id, e))?;

        // Compilation Devices
        let mut device_asts = HashMap::new();
        for (id, device_bundle) in &bundle.devices_resources {
            let device_content = device_bundle.scripts.get("main.rhai")
            .ok_or_else(|| format!("Device '{}': main.rhai not found", id))?;

            let ast = self.engine.compile(device_content)
            .map_err(|e| format!("Error compiling Device ({}): {}", id, e))?;

            device_asts.insert(id.clone(), ast);
        }

        Ok(CompiledDriver {
            driver: bundle.clone(),
           gateway_ast,
           device_asts,
        })
    }

    /// Injecte le contexte OSHEEMS dans Rhai
    fn build_context(&self, bundle: &ActiveDriver) -> Map {
        let mut context = Map::new();

        context.insert("interface".into(), self.bundle_to_rhai(&bundle.interface).into());
        context.insert("gateway".into(), self.bundle_to_rhai(&bundle.gateway).into());

        let mut devices_map = Map::new();
        for (id, device_bundle) in &bundle.devices_resources {
            devices_map.insert(id.clone().into(), self.bundle_to_rhai(device_bundle).into());
        }
        context.insert("devices".into(), devices_map.into());

        context
    }

    /// Convertit un ResourceBundle en Map Rhai (Compatible V2)
    fn bundle_to_rhai(&self, bundle: &ResourceBundle) -> Map {
        let mut map = Map::new();

        // 1. Nouvelle architecture : on injecte le contenu de definition (JSON) directement
        let template_def = rhai::serde::to_dynamic(bundle.template.clone()).unwrap_or_default();
        let mappings = rhai::serde::to_dynamic(bundle.mappings.clone()).unwrap_or_default();
        let config = rhai::serde::to_dynamic(bundle.configuration.clone()).unwrap_or_default();

        map.insert("template".into(), template_def);
        map.insert("mappings".into(), mappings);
        map.insert("config".into(), config.clone());

        // 2. Bloc de compatibilité "entity" pour les scripts existants
        // Permet d'accéder à entity.relation_attributes ou entity.config
        let mut entity = Map::new();
        let rel_attr = bundle.configuration.get("relation_attributes")
        .cloned()
        .unwrap_or(serde_json::json!({}));

        entity.insert("relation_attributes".into(), rhai::serde::to_dynamic(rel_attr).unwrap_or_default());
        entity.insert("config".into(), config);

        map.insert("entity".into(), entity.into());

        map
    }

    pub fn route(&self, compiled: &CompiledDriver, bundle: &ActiveDriver, payload: Map) -> Result<Dynamic, String> {
        let context = self.build_context(bundle);
        self.call(&compiled.gateway_ast, "on_data_received", vec![payload.into(), context.into()])
    }

    pub fn process_device(&self, compiled: &CompiledDriver, bundle: &ActiveDriver, device_id: &str, payload: Map) -> Result<Dynamic, String> {
        let ast = compiled.device_asts.get(device_id)
        .ok_or_else(|| format!("AST not found for device '{}'", device_id))?;
        let context = self.build_context(bundle);
        self.call(ast, "on_receive", vec![payload.into(), context.into()])
    }

    pub fn send_to_device(&self, compiled: &CompiledDriver, bundle: &ActiveDriver, device_id: &str, command_type: &str, command_value: Dynamic) -> Result<Map, String> {
        let context = self.build_context(bundle);
        let mut command_obj = Map::new();
        command_obj.insert("type".into(), command_type.into());
        command_obj.insert("value".into(), command_value);

        let result = self.call(&compiled.gateway_ast, "on_send_command", vec![device_id.into(), command_obj.into(), context.into()])?;
        result.try_cast::<Map>().ok_or_else(|| "on_send_command did not return a Map".to_string())
    }

    pub fn call(&self, ast: &AST, fn_name: &str, args: Vec<Dynamic>) -> Result<Dynamic, String> {
        let mut scope = Scope::new();
        self.engine.call_fn(&mut scope, ast, fn_name, args)
        .map_err(|e| format!("Rhai execution error in '{}': {}", fn_name, e))
    }
}
