# OSHEEMS - Common

## OSHEEMS - Software Architecture

### 1. Entity Types Reference (Database)
These types define the nature of the objects stored in the `entities` table.

| entity_type | Role / Description |
| :--- | :--- |
| **system** | Global system configuration, versioning, and core OSHEEMS parameters. |
| **user** | User profiles, authentication data, and access right definitions. |
| **interface** | Physical or logical link (e.g., `ttyNS0`, `eth0`, `i2c-1`). Handles raw byte streams. |
| **gateway** | Protocol entry point linking Interfaces to the system (e.g., Modbus Master, MQTT Broker). |
| **driver** | Handles data type conversions (Bin/Hex/Dec/Endianness) and executes Template scripts. |
| **device** | The specific hardware identified (e.g., a specific sensor, an energy meter). |
| **integration** | External software bridge (e.g., Home Assistant, Jeedom) communicating via standard protocols. |
| **group** | Logical collection of multiple devices for batch actions or shared properties. |
| **asset** | Major organizational units or physical holdings (e.g., Building, Vehicle Fleet). |
| **area** | Spatial or logical grouping structure within or across assets (e.g., Garage, Floor 1). |
| **controller** | Entity managing automation logic (e.g., Time schedulers, Scenarios). |
| **regulator** | Dynamic control logic (e.g., PID loops, load shedding algorithms). |

---

## 2. Manager Architecture (Rust Core)
Each manager is an autonomous module responsible for orchestrating a specific domain.

#### Core Data & System Managers
| Manager | Role / Responsibility |
| :--- | :--- |
| **DatabaseManager** | Handles SQLite persistence, migrations, and monthly data archiving. |
| **EntityManager** | Low-level CRUD operations for all entities and in-memory state caching. |
| **RelationManager** | Drives the relationship graph (From/Predicate/to, Calculations, Dependencies). |
| **TemplateManager** | Stores and compiles Templates (JSON, Rhai/JS scripts). |
| **CoreBusManager** | Orchestrates the System Bus (Internal MQTT) and message distribution. |
| **AuthManager** | Handles permissions, access tokens, and system bus security. |
| **SystemManager** | Manages global system state, updates, and overall health monitoring. |

#### Specialized Business Logic Managers
| Manager | Role / Responsibility |
| :--- | :--- |
| **InterfaceManager** | Commands hardware resources and manages raw byte-stream buffers. |
| **GatewayManager** | Orchestrates connections between Interfaces and Protocols (Ethernet, Serial, USB, etc.). |
| **DriverManager** | Instantiates drivers and executes scripts provided by Templates to normalize data. |
| **DeviceManager** | Manages hardware-specific lifecycle and status monitoring. |
| **IntegrationManager** | Manages bridges and discovery protocols for external platforms (Cloud, HA, Jeedom, Thingsboard). |
| **GroupManager** | Handles batch operations and logical grouping logic for devices. |
| **AssetManager** | Handles high-level asset management and global performance tracking. |
| **AreaManager** | Aggregates data at the zone level (e.g., total power consumption). |
| **ControllerManager** | Executes automated scenarios and event-based logic. |
| **RegulatorManager** | Runs real-time control loops (PID) and power regulation algorithms. |
| **UserManager** | Handles user-specific logic, preferences, and account management. |

---

## OSHEEMS - Directory Structure (src/common/managers/)

All managers follow the modular pattern: `mod.rs` for traits/types and `manager.rs` for implementation logic.

```text
src/common/
└── managers/
    ├── mod.rs              # Main entry point - exports all managers
    ├── system/             # Global system state & health
    │   ├── mod.rs
    │   └── manager.rs
    ├── user/               # User logic & profiles
    │   ├── mod.rs
    │   └── manager.rs
    ├── auth/               # Security, Tokens & RBAC
    │   ├── mod.rs
    │   └── manager.rs
    ├── entity/             # Low-level CRUD & Entity Caching
    │   ├── mod.rs
    │   └── manager.rs
    ├── relation/           # Graph logic (From/Predicate/to, Calculations)
    │   ├── mod.rs
    │   └── manager.rs
    ├── template/           # Template storage & Script compilation
    │   ├── mod.rs
    │   └── manager.rs
    ├── core_bus/           # Internal MQTT System Bus orchestration
    │   ├── mod.rs
    │   └── manager.rs
    ├── interface/          # Raw hardware transport (UART, I2C, SPI, Network)
    │   ├── mod.rs                    # Interface Trait
    │   ├── manager.rs                # Hardware Orchestrator
    │   ├── uart.rs                   # Serial implementation
    │   ├── i2c.rs                    # I2C implementation
    │   ├── spi.rs                    # SPI implementation
    │   └── network.rs                # Network socket implementation
    ├── gateway/            # Protocol & Interface orchestration
    │   ├── mod.rs
    │   └── manager.rs
    ├── driver/             # Data conversion (Bin/Hex/Dec) & Script execution
    │   ├── mod.rs
    │   └── manager.rs
    ├── device/             # Hardware lifecycle & Status
    │   ├── mod.rs
    │   └── manager.rs
    ├── integration/        # External bridges & discovery (Cloud, HA, Jeedom, Thingsboard)
    │   ├── mod.rs
    │   └── manager.rs
    ├── group/              # Logical batching of devices
    │   ├── mod.rs
    │   └── manager.rs
    ├── asset/              # High-level asset/holdings logic
    │   ├── mod.rs
    │   └── manager.rs
    ├── area/               # Spatial aggregation & Zone logic
    │   ├── mod.rs
    │   └── manager.rs
    ├── controller/         # Automation & Scenarios
    │   ├── mod.rs
    │   └── manager.rs
    ├── regulator/          # PID Loops & Power Regulation
    │   ├── mod.rs
    │   └── manager.rs
    └── database/           # SQLite connections & Archiving
        ├── mod.rs
        └── manager.rs
