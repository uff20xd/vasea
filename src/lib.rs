use std::{
    io::prelude::*,
    fs,
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
// (($r as byte)($g as byte)($b as byte))+
macro_rules! generate_task {
    () => {}
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct Pixel {
    r: Byte,
    g: Byte,
    b: Byte,
    // pub inner: Mutex<InnerPixel>
}

struct Task<F>
    where F: Fn(Pixel, usize, usize, usize, usize, f64, f64, f64) -> Pixel + 'static {
    function: &'static F,
    pixel: Pixel,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    zoom: f64,
    x_shift: f64,
    y_shift: f64,
}

#[derive(Copy, Clone)]
struct UnsafeImage(pub *mut [u8]);

unsafe impl Send for UnsafeImage {}
unsafe impl Sync for UnsafeImage {}

#[derive(Debug)]
pub struct Image {
    image: Vec<Byte>,
    dimensions: (usize, usize),
}

pub struct Shader<F> 
    where F: Fn(Pixel, usize, usize, usize, usize, f64, f64, f64) -> Pixel + 'static {
    // x, y, zoom, width, height
    pixel_fn: &'static F,
    zoom: f64,
    x_shift: f64,
    y_shift: f64,
    image: Rc<Image>,
}

impl Pixel {
    pub fn new(r: Byte, g: Byte, b: Byte) -> Self {
        Self {
            r,
            g,
            b,
        }
    }
    pub fn get_rgb(&self) -> (Byte, Byte, Byte){
        (self.r, self.g, self.b)
    }
}
impl<F> Task<F>
    where F: Fn(Pixel, usize, usize, usize, usize, f64, f64, f64) -> Pixel + 'static {
    pub fn new(
        function: &'static F,
        pixel: Pixel,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        zoom: f64,
        x_shift: f64,
        y_shift: f64,
    ) -> Task<F> {
        Self {
            function,
            pixel,
            x,
            y,
            width,
            height,
            zoom,
            x_shift,
            y_shift,
        }
    }

    #[inline(always)]
    fn execute(&self) -> Pixel {
        (self.function)(
            self.pixel,
            self.x,
            self.y,
            self.width,
            self.height,
            self.zoom,
            self.x_shift,
            self.y_shift,
        )
    }
}

impl Image {
    pub fn new(width: usize, height: usize, base: Byte) -> Self {
        Self {
            image: vec![base; width * height * 3],
            dimensions: (width, height),
        }
    }
    pub fn write<T>(&self, file_name: T) -> Result<(), Box<dyn std::error::Error>> 
    where T: AsRef<Path> {
        let image = &self.image;
        let mut file = fs::File::create(file_name.as_ref())?;
        let (width, height) = self.dimensions;

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
            dimensions: (width, height),
        })
    }
}

impl<F> Shader<F> 
    where F: Fn(Pixel, usize, usize, usize, usize, f64, f64, f64) -> Pixel + 'static + std::marker::Sync {

    pub fn new(
        pixel_fn: &'static F,
        zoom: f64,
        x_shift: f64,
        y_shift: f64,
        image: Image,
    ) -> Self { 
        Self {
            pixel_fn,
            zoom,
            x_shift,
            y_shift,
            image: image.into(),
        }
    }
    pub fn get_task(&self, x: usize, y: usize, pixel: Pixel) -> Task<F> {
        let (width, height) = self.image.dimensions;
        let task = Task::new(
            self.pixel_fn,
            pixel,
            x,
            y,
            width,
            height,
            self.zoom,
            self.x_shift,
            self.y_shift,
        );

        task
    }
    pub fn apply_shader(self, _thread_pool: &mut ThreadPool) -> Rc<Image> {
        {
            let mut image = UnsafeImage(&*self.image.image as *const [u8] as *mut [u8]);
            let mut x = 0;
            let mut y = 0;
            let (width, height) = self.image.dimensions;
            // let mut threads = Vec::new();
            for x in 0..width {
                for y in 0..height {
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
                    //threads.push(thread::spawn(move || {
                    // }));
                }
            }
            // threads.into_iter().map(|val| {
            //     _ = val.join().expect("[ERR] Couldnt do task");
            // });
        }
        self.image
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn todo() {
        todo!()
    }
}
