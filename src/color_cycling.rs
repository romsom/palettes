struct Palette {
	colors: [(u8, u8, u8)]
}

struct ColorCyclingImage<'a> {
	height: u32,
	width: u32,
	palettes: & 'a [& 'a Palette],
	data: & 'a [u8]
}

impl<'a> ColorCyclingImage<'a> {
	fn new(height: u32, width: u32, palettes: & 'a [& 'a Palette], data: & 'a [u8]) -> ColorCyclingImage<'a> {
		ColorCyclingImage {
			height: height,
			width: width,
			palettes: palettes,
			data: data
		}
	}
}
