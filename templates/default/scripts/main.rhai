// templates/default/scripts/main.rhai

// Pour une Interface : On renvoie la donnée brute (Passthrough)
fn handle_interface(data) {
    return data;
}

// Pour un Device : On renvoie l'état tel quel
fn handle_device(state) {
    return state;
}

// Pour une Gateway : Logique de routage par défaut
fn route_message(msg) {
    return msg;
}
