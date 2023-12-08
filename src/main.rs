use aws_sdk_s3::{Client, Error as S3Error, primitives::ByteStream};
use clap::Parser;
use std::{fs::File, io::Read, io::Write, path::Path};
use tracing::info;
use walkdir::WalkDir;
use zip::{ZipWriter, write::FileOptions};

/// Command-line arguments
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to the directory to zip
    #[clap(short, long)]
    criterion_dir: std::path::PathBuf,

    /// Name of the S3 bucket
    #[clap(short, long)]
    bucket: String,

    /// Object key for the uploaded file
    #[clap(short, long)]
    key: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    info!("Starting upload of benchmarks to S3");

    let subscriber = tracing_subscriber::fmt()
        // Use a more compact, abbreviated log format
        .compact()
        // Display the module path
        .with_target(true)
        // Display source code line numbers
        .with_line_number(true)
        // Display the thread ID an event was recorded on
        .with_thread_ids(true)
        // Don't display the event's target (module path)
        // Build the subscriber
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    let zip_file_path = "/tmp/criterion.zip";

    // Create a zip file
    let file = File::create(&zip_file_path)?;
    let mut zip = ZipWriter::new(file);

    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .unix_permissions(0o755);

    // Iterate over the files in the source directory
    for entry in WalkDir::new(&args.criterion_dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = path.strip_prefix(&args.criterion_dir).unwrap();

        if path.is_file() {
            zip.start_file(name.to_string_lossy(), options)?;
            let mut f = File::open(path)?;

            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            zip.write_all(&*buffer)?;
        }
        // Add directory support if needed
    }
    zip.finish()?;

    // Initialize the S3 client
    let shared_config = aws_config::load_from_env().await;
    let client = Client::new(&shared_config);

    // Read the zip file into a ByteStream
    let mut file = File::open(zip_file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let byte_stream = ByteStream::from(buffer);

    // Upload the zip file
    client.put_object()
        .bucket(&args.bucket)
        .key(&args.key)
        .body(byte_stream)
        .send()
        .await?;

    info!("Zip file uploaded to {} in bucket {}", args.key, args.bucket);
    Ok(())
}