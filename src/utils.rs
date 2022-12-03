extern crate image;
extern crate test;
#[allow(dead_code)]
#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::test::Bencher;
    use crate::string_path::StringPath;
    use std::io::Write;
    #[test]
    fn serialize_settings() {
        const SETTINGS_PATH: &str = "src/tests/settings_template.toml";
        let sp = StringPath::default();
        let mut file = std::fs::File::create(SETTINGS_PATH).unwrap();
        file.write_all(toml::to_string(&sp).unwrap().as_bytes())
            .unwrap();
    }
    #[test]
    fn deserialize_settings() {
        const SETTINGS_PATH: &str = "src/tests/settings.toml";
        let _sp = StringPath::from_file(SETTINGS_PATH).unwrap();
    }
}
