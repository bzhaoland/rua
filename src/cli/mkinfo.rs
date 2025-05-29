use clap::Args;

use crate::core::mkinfo;

#[derive(Args, Clone, Debug)]
pub(crate) struct MkinfoArgs {
    /// Build with IPv6 enabled
    #[arg(short = '6', long = "ipv6", default_value_t = false)]
    pub(crate) ipv6: bool,

    /// Enable coverage
    #[arg(short = 'g', long = "coverage", default_value_t = false)]
    pub(crate) coverage: bool,

    /// Enable coverity
    #[arg(short = 'c', long = "coverity", default_value_t = false)]
    pub(crate) coverity: bool,

    /// Build in debug mode
    #[arg(short = 'd', long = "debug", default_value_t = false)]
    pub(crate) debug: bool,

    /// Output format for makeinfos
    #[arg(long = "format", default_value = "list", value_name = "FORMAT")]
    pub(crate) output_format: mkinfo::DumpFormat,

    /// Build with shell password enabled
    #[arg(short = 'p', long = "password", default_value_t = false)]
    pub(crate) password: bool,

    /// Build with WebUI enabled
    #[arg(short = 'w', long = "webui")]
    pub(crate) webui: bool,

    /// Server to upload the output image to
    #[arg(short = 's', long = "image-server", value_name = "IMAGE-SERVER")]
    pub(crate) image_server: Option<mkinfo::ImageServer>,

    /// Binaries without stripping
    #[arg(long = "nostrip", value_name = "BINARY")]
    pub(crate) bins_without_strip: Vec<String>,

    /// Treat the positional arg as a build target other than a product name
    #[arg(long = "by-target")]
    pub(crate) by_target: bool,

    /// Product name like A1000, or compile target (when specify --by-target) like a-dnv.
    /// Can also be provided in regex like 'X\d+80' representing X6180/X7180/X8180, etc.
    #[arg(value_name = "NAME")]
    pub(crate) name: String,
}
