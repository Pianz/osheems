/* Gateway Script - MQTT
 * Handles inbound routing and outbound command translation
 */

// --- UPWARD DIRECTION: Core -> Gateway -> Network ---
fn on_send_command(device_id, command, context) {
    let devices = context.devices;

    if !devices.contains(device_id) {
        print_debug("Command Error: Device " + device_id + " not found in context.");
        return ();
    }

    let mqtt_id = devices[device_id].entity.relation_attributes.mqtt_id;
    let action = #{};

    // Translate logical command to physical MQTT parameters
    if command.type == "switch" {
        action.protocol = "mqtt";
        // REMOVED "osheems/" prefix to match external device expectations
        action.topic = "shellies/" + mqtt_id + "/command/switch:0";
        action.payload = if command.value { "on" } else { "off" };

        print_debug("Translating 'switch' command for " + device_id + " to " + action.payload);
        return action;
    }

    print_debug("Unknown command type: " + command.type);
    return ();
}

// --- DOWNWARD DIRECTION: Network -> Gateway -> Device ---
fn on_data_received(payload, context) {
    let sub_topic = payload.topic;

    for dev_id in context.devices.keys() {
        let mqtt_id = context.devices[dev_id].entity.relation_attributes.mqtt_id;

        // If the topic starts with our device identifier (e.g., "pv/...")
        if sub_topic.starts_with(mqtt_id) {
            return dev_id;
        }
    }

    print_debug("Routing Error: No device found for topic " + sub_topic);
    return "";
}
