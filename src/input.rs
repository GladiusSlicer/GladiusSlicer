use crate::utils::show_error_message;
use crate::*;
use std::path::PathBuf;
use std::str::FromStr;

/// The output of a file and a settings file
type FileOutput = Result<(Vec<(Vec<Vertex>, Vec<IndexedTriangle>)>, Settings), SlicerErrors>;

pub fn files_input(
    settings_path: Option<&str>,
    input: Option<Vec<String>>,
) -> FileOutput {
    info!("Loading Settings");
    let settings: Settings = {
        if let Some(str) = settings_path {
            load_settings(str)
        } else {
            Ok(Settings::default())
        }
    }?;

    info!("Loading Input");

    let converted_inputs: Vec<(Vec<Vertex>, Vec<IndexedTriangle>)> = input
        .ok_or(SlicerErrors::NoInputProvided)?
        .into_iter()
        .try_fold(vec![], |mut vec, value| {
            let model_path = Path::new(&value);

            debug!("Using input file: {:?}", model_path);

            let extension = model_path.extension().and_then(OsStr::to_str).ok_or(
                SlicerErrors::FileFormatNotSupported {
                    filepath: model_path.to_string_lossy().to_string(),
                },
            )?;

            let loader: Result<&dyn Loader, SlicerErrors> = match extension.to_lowercase().as_str()
            {
                "stl" => Ok(&STLLoader {}),
                "3mf" => Ok(&ThreeMFLoader {}),
                _ => Err(SlicerErrors::FileFormatNotSupported {
                    filepath: model_path.to_string_lossy().to_string(),
                }),
            };

            info!("Loading model from: {}", &value);

            let models = match loader?.load(model_path.to_str().ok_or(SlicerErrors::InputNotUTF8)?)
            {
                Ok(v) => v,
                Err(err) => {
                    show_error_message(err);
                    std::process::exit(-1);
                }
            };

            info!("Loading objects");
            let object = InputObject::Auto(value);

            let (x, y) = match object {
                InputObject::AutoTranslate(_, x, y) => (x, y),
                _ => (0.0, 0.0),
            };

            let transform = match object {
                InputObject::Raw(_, transform) => transform,
                InputObject::Auto(_) | InputObject::AutoTranslate(_, _, _) => {
                    let (min_x, max_x, min_y, max_y, min_z) =
                        models.iter().flat_map(|(v, _t)| v.iter()).fold(
                            (
                                f64::INFINITY,
                                f64::NEG_INFINITY,
                                f64::INFINITY,
                                f64::NEG_INFINITY,
                                f64::INFINITY,
                            ),
                            |a, b| {
                                (
                                    a.0.min(b.x),
                                    a.1.max(b.x),
                                    a.2.min(b.y),
                                    a.3.max(b.y),
                                    a.4.min(b.z),
                                )
                            },
                        );
                    Transform::new_translation_transform(
                        (x + settings.print_x - (max_x + min_x)) / 2.,
                        (y + settings.print_y - (max_y + min_y)) / 2.,
                        -min_z,
                    )
                }
            };

            let trans_str =
                serde_json::to_string(&transform).map_err(|_| SlicerErrors::InputMisformat)?;

            debug!("Using Transform {}", trans_str);

            vec.extend(models.into_iter().map(move |(mut v, t)| {
                for vert in &mut v {
                    *vert = &transform * *vert;
                }

                (v, t)
            }));

            Ok(vec)
        })?;
    Ok((converted_inputs, settings))
}

fn load_settings(filepath: &str) -> Result<Settings, SlicerErrors> {
    let settings_data =
        std::fs::read_to_string(filepath).map_err(|_| SlicerErrors::SettingsFileNotFound {
            filepath: filepath.to_string(),
        })?;
    let partial_settings: PartialSettingsFile =
        deser_hjson::from_str(&settings_data).map_err(|_| SlicerErrors::SettingsFileMisformat {
            filepath: filepath.to_string(),
        })?;
    let current_path = std::env::current_dir().map_err(|_| SlicerErrors::SettingsFilePermission)?;
    let mut path = PathBuf::from_str(filepath).map_err(|_| SlicerErrors::SettingsFileNotFound {
        filepath: filepath.to_string(),
    })?;

    path.pop();

    std::env::set_current_dir(&path).expect("Path checked before");

    let settings = partial_settings.get_settings()?;

    //reset path
    std::env::set_current_dir(current_path).expect("Path checked before");

    Ok(settings)
}
