use std::path::Path;
use image::image_dimensions;
use failure::Error;

/// Return the width and height of an image.
pub fn img_dimensions(path: &Path) -> Result<(u32, u32), Error>
{ return Ok(image_dimensions(path)?); }

#[cfg(test)]
mod tests {
    #[test]
    fn test_get_img_dimensions() {
        // TODO
        assert_eq!(2 + 2, 4);
    }
}
