#[derive(Clone, Copy)]
pub struct PathStep
{
    pub from_idx : usize,
    pub to_idx : usize,
    pub color_idx : usize,
    pub score : f32
}

#[derive(Default, Clone, PartialEq)]
pub enum StringCombo
{
    AllowedScored(f32),
    AllowedUnscored,
    #[default]
    Banned
}

#[derive(Default)]
pub struct StringPath
{
    pub path : Vec<PathStep>, //Each element of vector is (fron_index, to_index, color_index)
    pub pin_positions : Vec<(f32, f32)>, //Pin positions in unit space
    pin_radius : f32,
    input_image_path : String,
    input_image : Rgba32FImage, //Input image in Lab color space
    output_path : String,
    colors : Vec<Lab>,
    _background : Lab,
    path_length : usize,
    //Internally generated
    combo_scores : TriVec<Vec<StringCombo>>,
    strings_drawn : Rgba32FImage,
    cur_step : usize,
    cur_idxs : Vec<usize>,
    cur_scores : Vec<f32>,
    edge_weight : f32
}

