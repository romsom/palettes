extern crate sdl2;

mod color_cycling;
mod iff;

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;

use std::path::Path;

pub fn print_chunks(cs : & Vec<iff::IFFChunk>) {
	// recurse
	for chunk in cs.iter() {
		//print!("Chunk {}: Type: {}\n", chunk.chunk_number.unwrap(), chunk.chunk_type);
		print!("{}\n", chunk);
		print_chunks(& chunk.sub_chunks);
	}
}

pub fn main() {
	let iff_file = iff::IFFFile::read_from_file(Path::new("./V08AM.LBM"));
	print_chunks(& iff_file.chunks);


	let sdl_context = sdl2::init().unwrap();
	let video_subsystem = sdl_context.video().unwrap();

	let window = video_subsystem.window("rust-sdl2 demo", 800, 600)
		.position_centered()
		.build()
		.unwrap();

	let mut canvas = window.into_canvas().build().unwrap();

	canvas.set_draw_color(Color::RGB(0, 255, 255));
	canvas.clear();
	canvas.present();
	let mut event_pump = sdl_context.event_pump().unwrap();
	let mut i = 0u16;
	'running: loop {
		i = (i + 1) % 512;
		let ind: u8 = if i < 256 { i } else { 511 - i} as u8;
		let ind2: u8 = ((if ind < 128 { ind } else {255 - ind} as i8 * if i < 256 { -1 } else { 1 }) as i16 + 128i16) as u8;
		canvas.set_draw_color(Color::RGB(ind, ind2 / 2, 255 - ind));
		canvas.clear();
		for event in event_pump.poll_iter() {
			match event {
				Event::Quit {..} |
				Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
					break 'running
				},
				_ => {}
			}
		}
		// The rest of the game loop goes here...

		canvas.present();
		::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
	}
}
