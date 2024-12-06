use gladius_shared::settings::{PartialSettings, PartialSettingsFile};

#[test]
fn files_settings_validation() {
    let current_path = std::env::current_dir().unwrap();
    let mut settings_path = current_path.clone();
    settings_path.push("settings");
    std::env::set_current_dir(&settings_path).unwrap();
    let printers = vec!["Voron0.1.json", "CR10.json", "PrusaMk3.json"];
    let layers = vec![
        "0.1_layer_height.json",
        "0.2_layer_height.json",
        "0.3_layer_height.json",
    ];
    let filaments = vec![
        "ABS_Filament.json",
        "PETG_Filament.json",
        "TPU_Filament.json",
        "PLA_Filament.json",
    ];

    for printer in &printers {
        for layer in &layers {
            for filament in &filaments {
                println!("Testing {} {} {} ", printer, layer, filament);
                let psf = PartialSettingsFile {
                    other_files: Some(vec![
                        printer.to_string(),
                        layer.to_string(),
                        filament.to_string(),
                    ]),
                    partial_settings: PartialSettings::default(),
                };

                let result_settings = psf.get_settings(std::env::current_dir().unwrap());

                let settings = result_settings.unwrap();

                assert_eq!(
                    settings.validate_settings(),
                    gladius_shared::settings::SettingsValidationResult::NoIssue
                );
            }
        }
    }

    std::env::set_current_dir(current_path).expect("Must be run in correct environment");
}
