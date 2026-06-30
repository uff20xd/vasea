//extern vasae;
use vasea::*;
use shipped_shaders::editor::*;
use std::{
    io::prelude::*,
    fs,
    num::Wrapping,
    rc::Rc,
};
type Byte = u8;
const SCALE: usize = 1;
const XDIM: usize = SCALE * 16 * 40;
const YDIM: usize = SCALE * 16 * 40;
const CANVAS: &[u8] = include_bytes!("saul.ppm");
const EMINEM: &[u8] = include_bytes!("eminem_test.ppm");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = "./tests/saul.ppm";
    let out = "./out/output.ppm";
    let eminem = "./tests/eminem_test.ppm";
    let canvas = Image::parse_ppm(CANVAS)?;
    let in_image = Image::new(YDIM, XDIM , 255);
    let shader_range = ShaderRange::from_image(
        &canvas,
        RangeType::Percent,
        25.0,
        30.0,
        50.0,
        50.0,
        );
    let shader = Shader::new(
        &mandel_brot_shader,
        shader_range,
        Rc::new(
            MandelMetadata {
                width: canvas.width,
                height: canvas.height,
                zoom: 1200.2,
                x_shift: -0.01,
                y_shift: 0.64,
            }
        ),
        canvas 
    );
    let canvas = shader.apply_shader();

    let shader_range = ShaderRange::full(&canvas);
    let shader = Shader::new(
        &layer_shader,
        shader_range,
        Rc::new(
            LayerMetadata {
                width: canvas.width,
                height: canvas.height,
                zoom: 1.2,
                transparency: 0.5,
                image: Image::parse_ppm(EMINEM)?,
                shader_range,
                rotation: 0.0,
            }
        ),
        canvas
    );
    let canvas = shader.apply_shader();
    canvas.write(out);
    
    Ok(())
}

struct MandelMetadata {
    width: usize,
    height: usize,
    zoom: f64,
    x_shift: f64,
    y_shift: f64,
}

impl ShaderMetadata for MandelMetadata {}
// x, y, zoom, width, height
fn mandel_brot_shader(in_pixel: Pixel, x: usize, y: usize, metadata: Rc<MandelMetadata>) -> Pixel {
    let width = metadata.width;
    let height = metadata.height;
    let zoom = metadata.zoom;
    let x_shift = metadata.x_shift;
    let y_shift = metadata.y_shift;

    let (in_r, in_g, in_b) = in_pixel.get_rgb();
    if in_r < 230 ||
       in_g < 230 ||
       in_b < 230
    {
        return in_pixel;
    }
    let (in_r, in_g, in_b) = (255, 255, 255);
    if (height*x + y)%((width * height)/200) == 0 {
        println!("{}% finished!", ((((height*x + y) as f64 / (width * height) as f64)) * 100.0) as usize)
    }
    let zoom_mult = 1.0 / zoom;
    let scaling = width.min(height);
    let x0 = zoom_mult * (((x as f64/ scaling as f64)/ 2.0) - (x as f64/ scaling as f64)) + x_shift;
    let y0 = zoom_mult * (((y as f64/ scaling as f64)/ 2.0) - (y as f64/ scaling as f64)) + y_shift;
    // let r_mult: f64 = 0.1; 
    // let g_mult: f64 = 0.4 - (0.1_f64 * (x0 + y0 - 1.0)).abs();
    // let b_mult: f64 = 0.4 - (0.1_f64 * (x0 + y0 - 1.0)).abs() + 0.2 - (0.1_f64 * (x0 + y0 - 0.0)).abs();
    let r_mult: f64 = 0.4; 
    let g_mult: f64 = 0.4;
    let b_mult: f64 = 0.4;
    let max_mult = r_mult.max(g_mult.max(b_mult))-0.1;

    let mut x = 0.0;
    let mut y = 0.0;

    let mut colour: f64 = 0.0;
    let mut n = 0;
    let max_iteration = 40000;

    while (x*x + y*y <= 2.0*2.0) && (n < max_iteration) {
        let x_temp = x*x - y*y + x0;

        y = 2.0*x*y + y0;
        x = x_temp;

        n += 1;
    }

    //colour = (255 * (n/max_iteration)) as f64 / max_mult;
    colour = n as f64;

    if colour < 220.0 {
        colour = 0.0;
    }

    let r = Wrapping(in_r) - Wrapping((colour * r_mult).round() as Byte);
    let g = Wrapping(in_g) - Wrapping((colour * g_mult).round() as Byte);
    let b = Wrapping(in_b) - Wrapping((colour * b_mult).round() as Byte);

    // let r = (in_r as f64 * (colour * 0.2).round() as Byte as f64 / 255.0).round() as Byte;
    // let g = (in_g as f64 * (colour * 0.2).round() as Byte as f64 / 255.0).round() as Byte;
    // let b = (in_b as f64 * (colour * 0.2).round() as Byte as f64 / 255.0).round() as Byte;
    
    // Pixel::new(r, g, b)
    Pixel::new(r.0, g.0, b.0)
}
