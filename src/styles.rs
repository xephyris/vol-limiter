
use iced::Color;

pub fn get_rgb_color(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(r as f32 / 255f32, g as f32 / 255f32, b as f32 /255f32)
}
// #[allow(dead_code)]
pub fn get_rgba_color(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color::from_rgba(r as f32 / 255f32, g as f32 / 255f32, b as f32 /255f32, a as f32 / 255f32)
}
pub mod buttons {
    use iced::widget::button;
    use iced::{Border, Color};
    use super::*;

    #[allow(dead_code)]
    pub fn style_button(button_col: Color, text_col: Color, radius:i32) -> button::Style{
        button::Style{
            text_color: text_col,
            border: Border::default().rounded(radius),
            ..Default::default()
        }.with_background(button_col)
        
    }

    #[allow(dead_code)]
    pub fn style_from_rgba_button(r: u8, g: u8, b: u8, a: u8, text_col: Color, radius:i32) -> button::Style{
        button::Style{
            text_color: text_col,
            border: Border::default().rounded(radius),
            ..Default::default()
        }.with_background(get_rgba_color(r, g, b, a))
    }

    #[allow(dead_code)]
    pub fn style_from_rgb_button(r: u8, g: u8, b: u8, text_col: Color, radius:i32) -> button::Style{
        button::Style{
            text_color: text_col,
            border: Border::default().rounded(radius),
            ..Default::default()
        }.with_background(get_rgb_color(r, g, b))
    }
    
    
}