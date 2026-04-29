/* OSHEEMS Device Script - Shelly Pro 3EM
 *   Handles real-time metrics (em:0) and energy counters (emdata:0)
 */

fn on_receive(payload, context) {
    let sub_topic = payload.topic;
    let raw_data = parse_json(payload.data);
    let metrics = #{};

    print_debug("Decoding data from: " + sub_topic);

    // Détection du type de message basé sur la fin du topic
    if sub_topic.contains("status/em:0") {
        // --- Mapping des mesures en temps réel ---
        metrics.a_voltage = raw_data.a_voltage;
        metrics.a_current = raw_data.a_current;
        metrics.a_active_power = raw_data.a_act_power;

        metrics.b_voltage = raw_data.b_voltage;
        metrics.b_current = raw_data.b_current;
        metrics.b_active_power = raw_data.b_act_power;

        metrics.c_voltage = raw_data.c_voltage;
        metrics.c_current = raw_data.c_current;
        metrics.c_active_power = raw_data.c_act_power;

        metrics.total_active_power = raw_data.total_act_power;

    } else if sub_topic.contains("status/emdata:0") {
        // --- Mapping des compteurs d'énergie ---
        metrics.a_active_energy = raw_data.a_total_act_energy;
        metrics.b_active_energy = raw_data.b_total_act_energy;
        metrics.c_active_energy = raw_data.c_total_act_energy;

        metrics.total_active_energy = raw_data.total_act;
        metrics.total_returned_energy = raw_data.total_act_ret;
    }

    // On ajoute un timestamp interne pour le suivi
    metrics.timestamp_processed = now();

    // On retourne l'objet metrics pour qu'OSHEEMS puisse le publier
    return metrics;
}
