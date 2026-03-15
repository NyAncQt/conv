use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::process::{Command, exit};

#[derive(Debug, Clone, PartialEq)]
enum FileType {
    Audio,
    Video,
    Image,
    Document,
    Archive,
    Ebook,
    Unknown,
}

fn get_ext(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
}

fn classify(ext: &str) -> FileType {
    let audio = ["mp3","opus","ogg","wav","flac","aac","m4a","wma","aiff","alac","ape","mka","ra","amr","ac3","dts","mp2","m3u"];
    let video = ["mp4","mkv","avi","mov","webm","flv","wmv","m4v","ts","gif","3gp","mpeg","mpg","vob","ogv","rm","rmvb","divx","asf","m2ts","mts"];
    let image = ["jpg","jpeg","png","bmp","webp","tiff","tif","ico","gif","svg","avif","heic","heif","raw","cr2","nef","psd","xcf","pgm","ppm","pbm"];
    let document = ["pdf","docx","doc","odt","txt","md","html","htm","epub","rtf","tex","rst","csv","xlsx","xls","ods","pptx","ppt","odp","json","xml","yaml","yml"];
    let archive = ["zip","tar","gz","bz2","xz","7z","rar","zst","lz4","lzma","tgz","tbz2","iso"];
    let ebook = ["epub","mobi","azw","azw3","fb2","lit","lrf","pdb","cbz","cbr"];

    if audio.contains(&ext) { return FileType::Audio; }
    if video.contains(&ext) { return FileType::Video; }
    if image.contains(&ext) { return FileType::Image; }
    if ebook.contains(&ext) { return FileType::Ebook; }
    if document.contains(&ext) { return FileType::Document; }
    if archive.contains(&ext) { return FileType::Archive; }
    FileType::Unknown
}

fn check_tool(tool: &str) -> bool {
    Command::new("which")
        .arg(tool)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn require_tool(tool: &str) {
    if !check_tool(tool) {
        eprintln!("❌ Required tool '{}' not found. Install it first.", tool);
        exit(1);
    }
}

fn run(program: &str, args: &[&str]) -> i32 {
    let status = Command::new(program)
        .args(args)
        .status()
        .unwrap_or_else(|_| { eprintln!("Failed to run {}", program); exit(1); });
    status.code().unwrap_or(1)
}

fn convert_ffmpeg(input: &str, output: &str) -> i32 {
    require_tool("ffmpeg");
    run("ffmpeg", &["-i", input, "-y", output])
}

fn convert_pandoc(input: &str, output: &str) -> i32 {
    require_tool("pandoc");
    run("pandoc", &[input, "-o", output])
}

fn convert_imagemagick(input: &str, output: &str) -> i32 {
    require_tool("convert");
    run("convert", &[input, output])
}

fn convert_libreoffice(input: &str, output_ext: &str, output_dir: &str) -> i32 {
    require_tool("libreoffice");
    run("libreoffice", &["--headless", "--convert-to", output_ext, "--outdir", output_dir, input])
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: conv <input> <output>");
        eprintln!("Examples:");
        eprintln!("  conv song.mp4 song.mp3");
        eprintln!("  conv photo.png photo.jpg");
        eprintln!("  conv doc.docx doc.pdf");
        eprintln!("  conv video.mp4 video.gif");
        eprintln!("  conv book.epub book.mobi");
        exit(1);
    }

    let input = &args[1];
    let output = &args[2];

    if !Path::new(input).exists() {
        eprintln!("❌ Input file '{}' not found.", input);
        exit(1);
    }

    let in_ext = get_ext(input);
    let out_ext = get_ext(output);
    let in_type = classify(&in_ext);
    let out_type = classify(&out_ext);

    println!("🔄 Converting '{}' → '{}'", input, output);

    let code = match (&in_type, &out_type) {
        // Audio <-> Audio, Video <-> Audio, Video <-> Video
        (FileType::Audio, FileType::Audio) |
        (FileType::Video, FileType::Audio) |
        (FileType::Video, FileType::Video) |
        (FileType::Audio, FileType::Video) => convert_ffmpeg(input, output),

        // Image <-> Image
        (FileType::Image, FileType::Image) => {
            if check_tool("convert") {
                convert_imagemagick(input, output)
            } else {
                convert_ffmpeg(input, output)
            }
        }

        // Video -> Image (extract frame)
        (FileType::Video, FileType::Image) => {
            require_tool("ffmpeg");
            run("ffmpeg", &["-i", input, "-vframes", "1", "-y", output])
        }

        // Image -> Video (slideshow)
        (FileType::Image, FileType::Video) => convert_ffmpeg(input, output),

        // Document conversions
        (FileType::Document, FileType::Document) => {
            let pandoc_in = ["md","html","htm","rst","tex","docx","odt","rtf","epub","txt","csv","json","xml"];
            let pandoc_out = ["md","html","htm","rst","tex","docx","odt","rtf","epub","txt","pdf","json","xml"];
            let lo_in = ["docx","doc","odt","pptx","ppt","odp","xlsx","xls","ods","csv","rtf"];
            let lo_out = ["pdf","html","txt","odt","docx"];

            if pandoc_in.contains(&in_ext.as_str()) && pandoc_out.contains(&out_ext.as_str()) && check_tool("pandoc") {
                convert_pandoc(input, output)
            } else if lo_in.contains(&in_ext.as_str()) && lo_out.contains(&out_ext.as_str()) && check_tool("libreoffice") {
                let out_dir = Path::new(output).parent().and_then(|p| p.to_str()).unwrap_or(".");
                convert_libreoffice(input, &out_ext, out_dir)
            } else {
                eprintln!("❌ No converter available for {} → {}. Try installing pandoc or libreoffice.", in_ext, out_ext);
                1
            }
        }

        // Ebook conversions
        (FileType::Ebook, _) | (_, FileType::Ebook) => {
            if check_tool("ebook-convert") {
                run("ebook-convert", &[input, output])
            } else if check_tool("pandoc") {
                convert_pandoc(input, output)
            } else {
                eprintln!("❌ Install calibre (ebook-convert) for ebook conversion.");
                1
            }
        }

        // Archive extraction
        (FileType::Archive, _) => {
            eprintln!("❌ Archive extraction not supported as conversion. Use: tar, 7z, unzip, etc.");
            1
        }

        _ => {
            // Last resort: try ffmpeg
            eprintln!("⚠️  Unknown type combo ({} → {}), attempting ffmpeg...", in_ext, out_ext);
            convert_ffmpeg(input, output)
        }
    };

    if code == 0 {
        println!("✅ Done! Output: {}", output);
    } else {
        eprintln!("❌ Conversion failed with code {}", code);
        exit(code);
    }
}
