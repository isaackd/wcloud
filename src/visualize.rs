use serde_derive::Serialize;

#[derive(Serialize)]
pub enum Message {
    InitMessage(Init),
    ChangeWordMessage(Word),
    CheckRectMessage(CheckRect),
    PlacedWordMessage(PlaceWord),
}

#[derive(Serialize)]
pub struct Init {
    pub width: u32,
    pub height: u32,
    pub mask: Option<Vec<u8>>,
    pub font: Vec<u8>,
    pub background_color: [u8; 4],
}

#[derive(Serialize)]
pub struct CheckRect {
    pub x: u32,
    pub y: u32,
    pub empty: bool,
}

#[derive(Serialize)]
pub struct Word {
    pub text: String,
    pub font_size: u32,
    pub rect_width: u32,
    pub rect_height: u32,
    pub rotation: u32,
}

#[derive(Serialize)]
pub struct PlaceWord {
    pub text: String,
    pub font_size: u32,
    pub rotation: u32,
    pub x: u32,
    pub y: u32,
}


