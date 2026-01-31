use clap::{Parser, Subcommand, ValueEnum};
use srapi_rs::{FilebinProvider, FileProvider, JumpshareProvider, TempShProvider, TmpfilesProvider, UploadedFile};
use std::path::PathBuf;
use std::process;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Provider {
    Filebin,
    TempSh,
    Tmpfiles,
    Jumpshare,
}

#[derive(Parser)]
#[command(name = "srapi-rs", version, about = "Simple Rust API client for file providers")]
struct Cli {
    /// Select a provider backend
    #[arg(long, value_enum, default_value_t = Provider::Filebin)]
    provider: Provider,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new bin
    CreateBin,
    /// Upload a file to an existing bin
    Upload {
        /// Bin ID
        bin_id: String,
        /// Path to file
        file: PathBuf,
        /// Optional name to store in the bin (defaults to file name)
        #[arg(long)]
        name: Option<String>,
    },
    /// Upload a file using a non-bin provider
    UploadSimple {
        /// Path to file
        file: PathBuf,
        /// Optional name to store remotely (defaults to file name)
        #[arg(long)]
        name: Option<String>,
    },
    /// Fetch bin details and list files
    Details {
        /// Bin ID
        bin_id: String,
    },
    /// Fetch details for a direct file URL
    Info {
        /// File URL
        url: String,
    },
    /// Delete a file from a bin
    DeleteFile {
        /// Bin ID
        bin_id: String,
        /// File name
        name: String,
    },
    /// Delete an entire bin
    DeleteBin {
        /// Bin ID
        bin_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.provider {
        Provider::Filebin => {
            let provider = FilebinProvider::new();
            match cli.command {
                Commands::CreateBin => {
                    let bin_id = provider.create_bin().await?;
                    println!("{}", bin_id);
                }
                Commands::Upload { bin_id, file, name } => {
                    let file_name = resolve_name(&file, name)?;
                    let file_handle = File::open(&file).await?;
                    let metadata = file_handle.metadata().await?;
                    let stream = FramedRead::new(file_handle, BytesCodec::new());
                    let body = reqwest::Body::wrap_stream(stream);

                    provider
                        .upload_file(&bin_id, &file_name, body, metadata.len())
                        .await?;

                    println!("Uploaded {} to {}", file_name, bin_id);
                }
                Commands::Details { bin_id } => {
                    let details = provider.get_bin_details(&bin_id).await?;
                    println!("Bin: {}", details.id);
                    println!("Files: {}", details.file_count);
                    println!("Expires in: {}", details.expiration);
                    for file in details.files {
                        println!("- {} ({} bytes)", file.filename, file.size);
                    }
                }
                Commands::DeleteFile { bin_id, name } => {
                    provider.delete_file(&bin_id, &name).await?;
                    println!("Deleted {} from {}", name, bin_id);
                }
                Commands::DeleteBin { bin_id } => {
                    provider.delete_bin(&bin_id).await?;
                    println!("Deleted bin {}", bin_id);
                }
                Commands::UploadSimple { .. } | Commands::Info { .. } => {
                    eprintln!("Error: command not supported for filebin provider");
                    process::exit(2);
                }
            }
        }
        Provider::TempSh => {
            let provider = TempShProvider::new();
            handle_simple_provider(cli.command, provider, "temp.sh").await?;
        }
        Provider::Tmpfiles => {
            let provider = TmpfilesProvider::new();
            handle_simple_provider(cli.command, provider, "tmpfiles.org").await?;
        }
        Provider::Jumpshare => {
            let provider = JumpshareProvider::new();
            handle_simple_provider(cli.command, provider, "jumpshare.com").await?;
        }
    }

    Ok(())
}

fn resolve_name(file: &PathBuf, name: Option<String>) -> Result<String, Box<dyn std::error::Error>> {
    Ok(match name {
        Some(n) => n,
        None => file
            .file_name()
            .and_then(|f| f.to_str())
            .ok_or("File name missing or invalid")?
            .to_string(),
    })
}

async fn handle_simple_provider<T>(
    command: Commands,
    provider: T,
    label: &str,
) -> Result<(), Box<dyn std::error::Error>>
where
    T: SimpleUploadProvider,
{
    match command {
        Commands::UploadSimple { file, name } => {
            let file_name = resolve_name(&file, name)?;
            let bytes = tokio::fs::read(&file).await?;
            let uploaded = provider.upload_bytes(&file_name, bytes).await?;
            print_uploaded(uploaded, label);
        }
        Commands::Info { url } => {
            let info = provider.get_file_info(&url).await?;
            print_uploaded(info, label);
        }
        _ => {
            eprintln!("Error: unsupported command for {} provider", label);
            process::exit(2);
        }
    }
    Ok(())
}

fn print_uploaded(info: UploadedFile, label: &str) {
    println!("Provider: {}", label);
    println!("URL: {}", info.url);
    println!("Filename: {}", info.filename);
    println!("Size: {} bytes", info.size);
    println!("Content-Type: {}", info.content_type);
    if let Some(exp) = info.expires_at {
        println!("Expires at: {}", exp);
    }
}

#[async_trait::async_trait]
trait SimpleUploadProvider {
    async fn upload_bytes(&self, filename: &str, bytes: Vec<u8>) -> Result<UploadedFile, srapi_rs::ProviderError>;
    async fn get_file_info(&self, url: &str) -> Result<UploadedFile, srapi_rs::ProviderError>;
}

#[async_trait::async_trait]
impl SimpleUploadProvider for TempShProvider {
    async fn upload_bytes(&self, filename: &str, bytes: Vec<u8>) -> Result<UploadedFile, srapi_rs::ProviderError> {
        TempShProvider::upload_bytes(self, filename, bytes).await
    }

    async fn get_file_info(&self, url: &str) -> Result<UploadedFile, srapi_rs::ProviderError> {
        TempShProvider::get_file_info(self, url).await
    }
}

#[async_trait::async_trait]
impl SimpleUploadProvider for TmpfilesProvider {
    async fn upload_bytes(&self, filename: &str, bytes: Vec<u8>) -> Result<UploadedFile, srapi_rs::ProviderError> {
        TmpfilesProvider::upload_bytes(self, filename, bytes).await
    }

    async fn get_file_info(&self, url: &str) -> Result<UploadedFile, srapi_rs::ProviderError> {
        TmpfilesProvider::get_file_info(self, url).await
    }
}

#[async_trait::async_trait]
impl SimpleUploadProvider for JumpshareProvider {
    async fn upload_bytes(&self, filename: &str, bytes: Vec<u8>) -> Result<UploadedFile, srapi_rs::ProviderError> {
        JumpshareProvider::upload_bytes(self, filename, bytes).await
    }

    async fn get_file_info(&self, url: &str) -> Result<UploadedFile, srapi_rs::ProviderError> {
        JumpshareProvider::get_file_info(self, url).await
    }
}
