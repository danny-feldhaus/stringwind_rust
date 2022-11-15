use image::{Rgb};
use palette::{IntoColor, Srgb, Lab};

pub trait ImageToPalette
{
    fn as_srgb(&self) -> Srgb;
    fn as_lab(&self) -> Lab;
    fn lab_as_lab(&self) -> Lab;
    fn as_rgb(&self) -> Rgb<f32>;
}

impl ImageToPalette for Rgb<f32>
{
    fn as_srgb(&self) -> Srgb<f32>
    {
        return Srgb::from_components((self[0], self[1], self[2]));
    }
    fn as_lab(&self) -> Lab
    {
        return self.as_srgb().into_color();
    }
    fn as_rgb(&self) -> Rgb<f32>
    {
        return *self;
    }
    fn lab_as_lab(&self) -> Lab {
        return Lab::from_components((self[0], self[1], self[2]));
    }
}

impl ImageToPalette for Srgb<f32>
{
    fn as_srgb(&self) -> Srgb<f32>
    {
        return *self;
    }
    fn as_lab(&self) -> Lab
    {
        return (*self).into_color();
    }
    fn as_rgb(&self) -> Rgb<f32>
    {
        return Rgb([self.red, self.green, self.blue]);
    }
    //This shouldn't need to be used, because Srgb should intrinsically contain Srgb data
    fn lab_as_lab(&self) -> Lab {
        return Lab::from_components((self.red, self.green, self.blue));
    }
}

impl ImageToPalette for Lab
{
    fn as_srgb(&self) -> Srgb<f32>
    {
        return (*self).into_color();
    }
    fn as_lab(&self) -> Lab
    {
        return *self;
    }
    fn as_rgb(&self) -> Rgb<f32>
    {
        let srgb : Srgb = self.as_srgb();
        return Rgb([srgb.red, srgb.green, srgb.blue]);
    }
    fn lab_as_lab(&self) -> Lab {
        return *self;
    }
}

impl ImageToPalette for &[f32]
{
    fn as_srgb(&self) -> Srgb<f32>
    {
        return Srgb::from_components((self[0], self[1], self[2]));
    }
    fn as_lab(&self) -> Lab
    {
        return self.as_srgb().into_color();
    }
    fn as_rgb(&self) -> Rgb<f32>
    {
        return Rgb([self[0], self[1], self[2]]);
    }
    fn lab_as_lab(&self) -> Lab {
        return Lab::from_components((self[0], self[1], self[2]));
    }
}
