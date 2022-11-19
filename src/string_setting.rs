
use std::collections::HashMap;
use std::iter::repeat;
use palette::{Srgb, Lab, IntoColor};
use config::{Config, ConfigError};
pub trait StringSettingType
{
    fn get_setting<'a>(settings: &'a StringSettings, key: &str) -> Result<&'a Self, String>;
}
impl StringSettingType for usize
{
    fn get_setting<'a>(settings : &'a StringSettings, key: &str) -> Result<&'a Self, String>
    {
        match settings.size_vals.get(key)
        {
            Some(val) => Ok(val),
            None => Err(format!("Key {key} not present in settings."))
        }
    }

}
impl StringSettingType for String
{
    fn get_setting<'a>(settings : &'a StringSettings, key: &str) -> Result<&'a Self, String>
    {
        match settings.string_vals.get(key)
        {
            Some(val) => Ok(val),
            None => Err(format!("Key {key} not present in settings."))
        }    
    }
}
impl StringSettingType for f32
{
    fn get_setting<'a>(settings : &'a StringSettings, key: &str) -> Result<&'a Self, String>
    {
        match settings.float_vals.get(key)
        {
            Some(val) => Ok(val),
            None => Err(format!("Key {key} not present in settings."))
        }   
     }
}
impl StringSettingType for Lab
{
    fn get_setting<'a>(settings : &'a StringSettings, key: &str) -> Result<&'a Self, String>
    {  
        match settings.lab_vals.get(key)
        {
            Some(val) => Ok(val),
            None => Err(format!("Key {key} not present in settings."))
        }
    }
}
impl StringSettingType for Vec<Lab>
{
    fn get_setting<'a>(settings : &'a StringSettings, key: &str) -> Result<&'a Self, String>
    {
        match settings.lab_vec_vals.get(key)
        {
            Some(val) => Ok(val),
            None => Err("Key {key} not present in settings.".to_string())
        }
    }
}

pub struct StringSettings
{
    cfg: Config,
    size_vals: HashMap<&'static str, usize>,
    string_vals: HashMap<&'static str, String>,
    float_vals: HashMap<&'static str, f32>,
    lab_vals: HashMap<&'static str, Lab>,
    lab_vec_vals: HashMap<&'static str, Vec<Lab>>
}

impl StringSettings
{
    pub fn get<T: StringSettingType>(&self,key: &str) -> Result<&T, String>
    {
        return T::get_setting(self, key);
    }

}

impl Default for StringSettings
{
    fn default() -> Self {
        let usize_keys = ["pin_count", "line_count"];
        let string_keys = ["in_image_path", "out_image_path"];
        let float_keys = ["pin_radius"];
        let lab_keys = ["bg_color"];
        let lab_vec_keys = ["str_colors"];

        return StringSettings {
            cfg: Config::default(),
            size_vals:  itertools::zip(usize_keys, repeat(usize::default())).collect(),
            string_vals:  itertools::zip(string_keys, repeat(String::default())).collect(),
            float_vals: itertools::zip(float_keys, repeat(f32::default())).collect(),
            lab_vals:  itertools::zip(lab_keys, repeat(Lab::default())).collect(),
            lab_vec_vals: itertools::zip(lab_vec_keys, repeat(Vec::<Lab>::default())).collect()
        };
    }
}

/*Attempts to read each default setting key from the given file

Returns:
    Ok => A StringSettings containing the read in data
    Err => An error regarding either a file or key error.
 */

pub fn read_string_settings(path : &'static str) -> Result<StringSettings, ConfigError>
{
    let mut ss: StringSettings = StringSettings::default();
    let cfg = Config::builder()
        .add_source(config::File::with_name(path))
        .build()?;

    ss.cfg = cfg;

    for (key, val) in ss.size_vals.iter_mut()
    {
        *val = ss.cfg.get_int(key)? as usize;
    }
    for (key, val) in ss.string_vals.iter_mut()
    {
        *val = ss.cfg.get_string(key)?;
    }
    for(key, val) in ss.float_vals.iter_mut()
    { 
        *val =  ss.cfg.get_float(key)? as f32;
    }
    for(key, val) in ss.lab_vals.iter_mut()
    {
        *val = parse_lab_color(&ss.cfg.get::<config::Value>(key)?)?;
    }
    for(key, val) in ss.lab_vec_vals.iter_mut()
    {
        *val = parse_lab_colors(&ss.cfg.get_array(key)?)?;
    }
    Ok(ss)
}
fn parse_lab_colors(val_vec: &Vec<config::Value>) -> Result<Vec<Lab>, ConfigError>
{
    let mut col_vec: Vec<Lab> = vec![Lab::new(0.,0.,0.);val_vec.len()];
    for i in 0..val_vec.len()
    {
        col_vec[i] = parse_lab_color(&val_vec[i])?;
    }
    Ok(col_vec)
}

fn parse_lab_color(s_color_rgb: &config::Value) -> Result<Lab, ConfigError>
{
    Ok(parse_rgb_color(s_color_rgb)?.into_color())
}

fn parse_rgb_color(s_color: &config::Value) -> Result<Srgb, ConfigError>
{
    let c_arr = s_color.clone().into_array()?;
    Ok(Srgb::new(
        c_arr[0].clone().into_float()? as f32,
        c_arr[1].clone().into_float()? as f32,
        c_arr[2].clone().into_float()? as f32
    ))
}

