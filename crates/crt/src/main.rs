mod threads;

use std::{
    io::{self, Read},
    num::NonZeroUsize,
};

use anyhow::Context;
use render::rgb;
use threads::Threads;

/// Renders an image in ppm format.
#[derive(argh::FromArgs)]
struct Args {
    /// amount of parallelism, defaults to the number of cores
    #[argh(option, short = 'j')]
    jobs: Option<NonZeroUsize>,

    /// memory to use, in kilobytes
    #[argh(option, default = "640")]
    mem: usize,

    /// width of the image, in pixels
    #[argh(option, default = "800")]
    width: u32,

    /// height of the image, in pixels
    #[argh(option, default = "600")]
    height: u32,
}

fn main() -> anyhow::Result<()> {
    let args: Args = argh::from_env();

    let mut crt = String::new();
    io::stdin().read_to_string(&mut crt).context("reading input")?;

    let mut mem = vec![0; args.mem * 1024];
    let threads = match args.jobs {
        Some(it) => Threads::new(it),
        None => Threads::with_max_threads()?,
    };
    let mut buf = vec![rgb::Color::default(); (args.width * args.height) as usize];
    let mut buf = rgb::Buf::new([args.width, args.height], &mut buf);

    render::render(&crt, &mut mem, &|f| threads.in_parallel(f), &mut buf)
        .map_err(|err| anyhow::format_err!("{err}"))?;

    write_ppm(&buf, &mut io::stdout().lock()).context("writing output")?;
    Ok(())
}

fn write_ppm(buf: &rgb::Buf, w: &mut dyn io::Write) -> io::Result<()> {
    let magic_number = "P3";
    let max_color = 255;
    write!(w, "{}\n{} {}\n{}\n", magic_number, buf.width(), buf.height(), max_color)?;

    for idx in buf.by_row() {
        if idx[0] == 0 {
            write!(w, "\n")?;
        }
        let rgb::Color { r, g, b } = buf[idx];
        write!(w, "{r:3} {g:3} {b:3}  ")?;
    }
    Ok(())
}
