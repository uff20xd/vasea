use std::{
    io::prelude::*, fs,
    rc::Rc,
    ffi::OsStr,
    path::PathBuf,
    path::Path,
    thread,
};
type Byte = u8;

pub mod thread_pool;

use thread_pool::*;

// PPM Format:
// "P6" \n
// $width $height\n
// $max_colour_component_value\n

#[derive(Copy, Clone)]
struct UnsafeImage(pub *mut [u8]);

unsafe impl Send for UnsafeImage {}
unsafe impl Sync for UnsafeImage {}

pub trait ShaderMetadata {}


#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct Pixel {
    r: Byte,
    g: Byte,
    b: Byte,
    // pub inner: Mutex<InnerPixel>
}

impl Pixel {
    #[inline(always)]
    pub fn new(r: Byte, g: Byte, b: Byte) -> Self {
        Self {
            r,
            g,
            b,
        }
    }
    #[inline(always)]
    pub fn get_rgb(&self) -> (Byte, Byte, Byte){
        (self.r, self.g, self.b)
    }
}

struct Task<F, METADATA>
    where METADATA: ShaderMetadata,
        F: Fn(Pixel, usize, usize, Rc<METADATA>) -> Pixel + 'static {
    function: &'static F,
    pixel: Pixel,
    x: usize,
    y: usize,
    metadata: Rc<METADATA>,
}
impl<F, METADATA> Task<F, METADATA>
    where METADATA: ShaderMetadata,
        F: Fn(Pixel, usize, usize, Rc<METADATA>) -> Pixel + 'static {
    pub fn new(
        function: &'static F,
        pixel: Pixel,
        x: usize,
        y: usize,
        metadata: Rc<METADATA>,
    ) -> Task<F, METADATA> {
        Self {
            function,
            pixel,
            x,
            y,
            metadata,
        }
    }

    #[inline(always)]
    fn execute(&self) -> Pixel {
        (self.function)(
            self.pixel,
            self.x,
            self.y,
            self.metadata.clone(),
        )
    }
}

#[derive(Debug)]
pub struct Image {
    image: Vec<Byte>,
    pub width: usize,
    pub height: usize,
}
impl Image {
    pub fn new(width: usize, height: usize, base: Byte) -> Self {
        Self {
            image: vec![base; width * height * 3],
            width,
            height,
        }
    }
    pub fn write<T>(&self, file_name: T) -> Result<(), Box<dyn std::error::Error>> 
    where T: AsRef<Path> {
        let image = &self.image;
        let mut file = fs::File::create(file_name.as_ref())?;
        let width = self.width;
        let height = self.height;

        let size: Vec<u8> = format!("{} {}\n", width, height).bytes().collect();
        _ = file.write(&(b"P6\n")[..]);
        _ = file.write(&size[..]);
        _ = file.write(&(b"255\n")[..]);

        file.write(&image[..]);
        Ok(())
    }
    pub fn read_ppm<T>(file_name: T) -> Result<Self, Box<dyn std::error::Error>>
    where T: AsRef<Path> {
        let mut file = fs::read(file_name.as_ref())?;
        let mut index = 3;
        let mut raw_numbers = String::new();
        loop {
            if file[index] == 10 { break; }
            raw_numbers.push(file[index] as char);
            index += 1;
        }
        index += 5;
        let image_data = Vec::from(&file[index..]);
        let mut raw_number_split = raw_numbers.split_whitespace();
        let raw_width = raw_number_split.next().expect("[ERR] width");
        let raw_height = raw_number_split.next().expect("[ERR] height");
        dbg!(&raw_width);
        dbg!(&raw_height);
        let width: usize = raw_width.parse()?;
        let height: usize = raw_height.parse()?;
        Ok(Self {
            image: image_data,
            width,
            height,
        })
    }
}

pub struct Shader<F, METADATA>
    where METADATA: ShaderMetadata,
        F: Fn(Pixel, usize, usize, Rc<METADATA>) -> Pixel + 'static {
    // x, y, zoom, width, height
    pixel_fn: &'static F,
    metadata: Rc<METADATA>,
    image: Rc<Image>,
    shader_range: ShaderRange,
}

impl<F, METADATA> Shader<F, METADATA> 
    where METADATA: ShaderMetadata,
        F: Fn(Pixel, usize, usize, Rc<METADATA>) -> Pixel + 'static {

    pub fn new(
        pixel_fn: &'static F,
        shader_range: ShaderRange,
        metadata: Rc<METADATA>,
        image: Image,
    ) -> Self { 
        Self {
            pixel_fn,
            metadata,
            image: image.into(),
            shader_range,
        }
    }
    #[inline(always)]
    pub fn get_task(&self, x: usize, y: usize, pixel: Pixel) -> Task<F, METADATA> {
        let width = self.image.width;
        let height = self.image.height;
        let task = Task::new(
            self.pixel_fn,
            pixel,
            x,
            y,
            self.metadata.clone(),
        );

        task
    }
    pub fn apply_shader(self) -> Rc<Image> {
        let mut image = UnsafeImage(&*self.image.image as *const [u8] as *mut [u8]);
        let width = self.image.width;
        let height = self.image.height;
        // let mut threads = Vec::new();
        let width_range = self.shader_range.x_len;
        let height_range = self.shader_range.y_len;
        for x in 0..width_range {
            for y in 0..height_range {
                let x = x + self.shader_range.x_offset;
                let y = y + self.shader_range.y_offset;
                let pixel;
                unsafe {
                    pixel = Pixel::new(
                        (*(image.0))[(y*width + x)*3],
                        (*(image.0))[(y*width + x)*3 + 1],
                        (*(image.0))[(y*width + x)*3 + 2]);
                }
                let task = self.get_task(x, y, pixel);
                let (r,g,b) = task.execute().get_rgb();
                let wrapped_image = image;
                let p_image = wrapped_image.0;
                unsafe {
                    (&mut *p_image)[(y*width + x)*3] = r;
                    (&mut *p_image)[(y*width + x)*3 + 1] = g;
                    (&mut *p_image)[(y*width + x)*3 + 2] = b;
                }
            }
        }
        self.image
    }
}

#[derive(Clone, Copy)]
pub enum RangeType {
    Percent,
    Pixel,
}

#[derive(Clone, Copy)]
pub struct ShaderRange {
    x_offset: usize,
    y_offset: usize,
    x_len: usize,
    y_len: usize,
}

impl ShaderRange {
    pub fn new(
        x_offset: usize,
        y_offset: usize,
        x_len: usize,
        y_len: usize,
    ) -> Self {
        Self {
            x_offset,
            y_offset,
            x_len,
            y_len,
        }
    }
    pub fn from_image(
        image: &Image,
        range_type: RangeType,
        x_offset: f64,
        y_offset: f64,
        x_len: f64,
        y_len: f64,
    ) -> Self {
        match range_type {
            RangeType::Percent => {
                let x_offset = ((x_offset / 100.0) * image.width as f64) as usize;
                let y_offset = ((y_offset / 100.0) * image.height as f64) as usize;
                let x_len = ((x_len / 100.0) * image.width as f64) as usize;
                let y_len = ((y_len / 100.0) * image.height as f64) as usize;
                Self {
                    x_offset,
                    y_offset,
                    x_len,
                    y_len,
                }
            },
            RangeType::Pixel => {
                let x_offset = x_offset as usize;
                let y_offset = y_offset as usize;
                let x_len = x_len as usize;
                let y_len = y_len as usize;
                Self {
                    x_offset,
                    y_offset,
                    x_len,
                    y_len,
                }
            },
        }
    }
    pub fn from_image_duo(
        image: &Image,
        range_type: RangeType,
        x_offset: f64,
        y_offset: f64,
        overlay_image: &Image,
    ) -> Self {
        match range_type {
            RangeType::Percent => {
                let x_offset = ((x_offset / 100.0) * x_offset) as usize;
                let y_offset = ((y_offset / 100.0) * y_offset) as usize;
                let x_len = overlay_image.width;
                let y_len = overlay_image.height;
                Self {
                    x_offset,
                    y_offset,
                    x_len,
                    y_len,
                }
            },
            RangeType::Pixel => {
                let x_offset = x_offset as usize;
                let y_offset = y_offset as usize;
                let x_len = overlay_image.width;
                let y_len = overlay_image.height;
                Self {
                    x_offset,
                    y_offset,
                    x_len,
                    y_len,
                }
            },
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn todo() {
        todo!()
    }
}
