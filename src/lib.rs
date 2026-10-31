use std::fmt::Debug;

use image::ImageReader;

use std::io::Write;

const CHARACTER_WIDTH: usize = 12;
const CHARACTER_HEIGHT: usize = 18;
const CHARACTERS_PER_ROW: usize = 16;
const ROW_COUNT: usize = 256 / CHARACTERS_PER_ROW;

const EXPECTED_IMAGE_WIDTH: usize = (CHARACTERS_PER_ROW * (CHARACTER_WIDTH + 1)) + 1;
const EXPECTED_IMAGE_HEIGHT: usize = (ROW_COUNT * (CHARACTER_HEIGHT + 1)) + 1;

impl Debug for Png2Max7456Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IOError(arg0) => f.debug_tuple("IOError").field(arg0).finish(),
            Self::ImageError(arg0) => f.debug_tuple("ImageError").field(arg0).finish(),
            Self::WrongImageDimensions(width, height) => {
                write!(
                    f,
                    "WrongImageDimensions(Expected width={} height>={}; got width={} height={})",
                    EXPECTED_IMAGE_WIDTH, EXPECTED_IMAGE_HEIGHT, width, height
                )
            }
        }
    }
}

pub enum Png2Max7456Error {
    IOError(std::io::Error),
    ImageError(image::ImageError),
    WrongImageDimensions(u32, u32),
}

impl From<std::io::Error> for Png2Max7456Error {
    fn from(value: std::io::Error) -> Self {
        Self::IOError(value)
    }
}

impl From<image::ImageError> for Png2Max7456Error {
    fn from(value: image::ImageError) -> Self {
        Self::ImageError(value)
    }
}

#[derive(Clone, Copy, Default)]
enum Pixel {
    White,
    Black,
    #[default]
    Transparent,
}

impl From<u8> for Pixel {
    fn from(grayscale: u8) -> Self {
        if grayscale > 0xf8 {
            Pixel::White
        } else if grayscale < 0x08 {
            Pixel::Black
        } else {
            Pixel::Transparent
        }
    }
}

impl From<Pixel> for u8 {
    fn from(pixel: Pixel) -> u8 {
        match pixel {
            Pixel::White => 0b10,
            Pixel::Black => 0b00,
            Pixel::Transparent => 0b01,
        }
    }
}

impl Debug for Pixel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Pixel::White => '#',
            Pixel::Black => '|',
            Pixel::Transparent => 'O',
        };

        write!(f, "{}", value)
    }
}

#[derive(Default, Clone, Copy)]
struct Character {
    pixels: [[Pixel; CHARACTER_WIDTH]; CHARACTER_HEIGHT],
}

impl Debug for Character {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for row in self.pixels {
            for character in row {
                write!(f, "{:?}", character)?;
            }
            writeln!(f)?
        }

        Ok(())
    }
}

impl Character {
    fn as_bytes(&self) -> [u8; 64] {
        let mut bytes = [0u8; 64];
        let mut pair_index = 0;
        for row in self.pixels {
            for pixel in row {
                let pixel_value: u8 = pixel.into();

                let byte_index = pair_index / 4;
                let pair_offset = 8 - (pair_index % 4 + 1) * 2;
                bytes[byte_index] |= pixel_value << pair_offset;

                pair_index += 1;
            }
        }

        bytes
    }
}

pub struct FontImage {
    characters: [Character; 256],
}

impl FontImage {
    pub fn load(path: &str) -> Result<Self, Png2Max7456Error> {
        let image = ImageReader::open(path)?.decode()?;
        let image = image.into_luma8();

        if (image.width() != (EXPECTED_IMAGE_WIDTH as u32))
            || (image.height() < (EXPECTED_IMAGE_HEIGHT as u32))
        {
            return Err(Png2Max7456Error::WrongImageDimensions(
                image.width(),
                image.height(),
            ));
        }

        let mut characters = [Character::default(); 256];

        for (index, character) in characters.iter_mut().enumerate() {
            let row = index / CHARACTERS_PER_ROW;
            let column = index % CHARACTERS_PER_ROW;

            let column_offset = column * (CHARACTER_WIDTH + 1) + 1;
            let row_offset = row * (CHARACTER_HEIGHT + 1) + 1;

            for y in 0..CHARACTER_HEIGHT {
                for x in 0..CHARACTER_WIDTH {
                    let pixel = image[((x + column_offset) as u32, (y + row_offset) as u32)].0[0];
                    character.pixels[y][x] = pixel.into();
                }
            }
        }

        Ok(Self { characters })
    }

    pub fn write_to_rust_file(
        &self,
        file: &mut impl Write,
        variable_name: &str,
    ) -> Result<(), Png2Max7456Error> {
        write!(file, "pub const {variable_name}: [[u8; 64];256] = [")?;
        write!(file, "{:?}", self.characters[0].as_bytes())?;
        for character in 1..256 {
            write!(file, ", {:?}", self.characters[character].as_bytes())?;
        }
        write!(file, "];")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    mod test_data;
    use crate::FontImage;

    #[test]
    fn test_image_processing() {
        let image = FontImage::load("test_assets/sample_image.png").unwrap();
        for character in 0..256 {
            assert_eq!(
                image.characters[character].as_bytes(),
                test_data::EXPECTED_CHARACTER_MEMORY[character]
            );
        }

        let mut rust_file_content: Vec<u8> = Vec::new();
        image
            .write_to_rust_file(&mut rust_file_content, "FONT_DATA")
            .unwrap();
        let written = String::from_utf8(rust_file_content).unwrap();
        let expected = format!(
            "pub const FONT_DATA: [[u8; 64];256] = {:?};",
            test_data::EXPECTED_CHARACTER_MEMORY
        );
        assert_eq!(written, expected);
    }
}
