use clap::Args;

#[derive(Args, Clone, Debug)]
pub(crate) struct ReviewArgs {
    #[arg(
        short = 'n',
        long = "bug",
        value_name = "BUG",
        help = "Bug id for this review request (required)"
    )]
    pub(crate) bug_id: u32,

    #[arg(
        short = 'r',
        long = "review-id",
        value_name = "REVIEW-ID",
        help = "Existing review id"
    )]
    pub(crate) review_id: Option<u32>,

    #[arg(
        short = 'd',
        long = "diff-file",
        value_name = "DIFF-FILE",
        help = "Diff file to be used"
    )]
    pub(crate) diff_file: Option<String>,

    #[arg(
        short = 'u',
        long = "reviewers",
        value_name = "REVIEWERS",
        help = "Reviewers"
    )]
    pub(crate) reviewers: Option<Vec<String>>,

    #[arg(
        short = 'b',
        long = "branch",
        value_name = "BRANCH",
        help = "Branch name for this commit"
    )]
    pub(crate) branch_name: Option<String>,

    #[arg(
        short = 'p',
        long = "repo",
        value_name = "REPO",
        help = "Repository name"
    )]
    pub(crate) repo_name: Option<String>,

    #[arg(
        short = 's',
        long = "revision",
        value_name = "REVISION",
        help = "Revision to be used"
    )]
    pub(crate) revisions: Option<String>,

    #[arg(
        short = 't',
        long = "template-file",
        value_name = "TEMPLATE-FILE",
        help = "Use customized template file (please ensure it can run through svn commit hooks)"
    )]
    pub(crate) template_file: Option<String>,

    #[arg(value_name = "FILE", help = "Files to be reviewed")]
    pub(crate) files: Option<Vec<String>>,
}
