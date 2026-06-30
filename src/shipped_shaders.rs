pub mod editor {
    use crate::*;
    pub struct LayerMetadata {
        pub width: usize,
        pub height: usize,
        pub zoom: f64,
        pub transparency: f64,
        pub image: Image,
        pub shader_range: ShaderRange,
        pub rotation: f64,
    }

    impl ShaderMetadata for LayerMetadata {}

    pub fn layer_shader(in_pixel: Pixel, raw_x: usize, raw_y: usize, metadata: Rc<LayerMetadata>) -> Pixel {
        let width = metadata.width;
        let height = metadata.height;
        let zoom = metadata.zoom;
        let transparency = metadata.transparency;
        let x = raw_x;
        let y = raw_y;
        if x >= (metadata.image.width  as f64 * zoom) as usize ||
            y >= (metadata.image.height  as f64 * zoom) as usize {
                return in_pixel;
        }
        let pixel_num = (
            (y as f64 / zoom) as usize *
            metadata.image.width +
            (x as f64 / zoom) as usize
        );

        let r = metadata.image.image[pixel_num*3];
        let g = metadata.image.image[pixel_num*3 + 1];
        let b = metadata.image.image[pixel_num*3 + 2];
        let pixel;
        if transparency >= 1.0 {
            pixel = Pixel::new(r, g, b);
        }
        else if transparency <= 0.0 {
            pixel = in_pixel;
        }
        else {
            let (or, og, ob) = in_pixel.get_rgb();

            pixel = Pixel::new(
                (or as f64 * (1.0 - transparency) + r as f64 * transparency) as u8,
                (og as f64 * (1.0 - transparency) + g as f64 * transparency) as u8,
                (ob as f64 * (1.0 - transparency) + b as f64 * transparency) as u8,
            );
        }
        pixel
    }
}

