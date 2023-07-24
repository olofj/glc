use structopt::StructOpt;

mod commands {
    pub mod credentials;
    pub mod job;
    pub mod list_jobs;
    pub mod list_pipelines;
    pub mod list_projects;
    pub mod login;
    pub mod pipeline;
    pub mod show_job;
}

use commands::credentials::load_credentials;
use commands::list_jobs::list_jobs;
use commands::list_pipelines::list_pipelines;
use commands::list_projects::list_projects;
use commands::login::login;
use commands::show_job::show_job;

#[derive(StructOpt, Debug)]
#[structopt(name = "glc", about = "gitlab client utility")]
struct Opt {
    /// The project ID
    #[structopt(short = "P", long = "project", env = "GITLAB_PROJECT")]
    project: Option<String>,

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt, Debug)]
enum Command {
    // ...
    /// Login to GitLab
    #[structopt(name = "login")]
    Login {
        /// Personal access token
        #[structopt(short = "t", long = "token", parse(from_str))]
        token: String,

        /// GitLab URL
        #[structopt(short = "u", long = "url", parse(from_str))]
        url: String,
    },
    /// List jobs
    #[structopt(name = "list-jobs")]
    ListJobs {
        /// Pipeline ID to list jobs for
        #[structopt(short = "p", long = "pipeline")]
        pipeline: Option<usize>,
    },
    /// List projects
    #[structopt(name = "list-projects")]
    ListProjects {},
    /// Show job
    #[structopt(name = "show-job")]
    ShowJob {
        /// The ID of the job to show
        #[structopt()]
        job: usize,
        /// Status summary after output
        #[structopt(long = "no-status", parse(from_flag = std::ops::Not::not))]
        status: bool,
        /// Follow (keep listening)
        #[structopt(short = "f", long = "follow")]
        follow: Option<bool>,
        /// Number of lines of output to show (negative number)
        #[structopt()]
        tail: Option<isize>,
    },
    /// List pipelines
    #[structopt(name = "list-pipelines")]
    ListPipelines {},
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    let project = opt.project.unwrap_or_default();

    match opt.cmd {
        Command::Login { token, url } => {
            login(&token, &url)?;
        }
        Command::ListJobs { pipeline } => {
            let creds = load_credentials()?;
            list_jobs(&creds, &project, pipeline).await?;
        }
        Command::ShowJob {
            job,
            status,
            follow,
            tail,
        } => {
            let creds = load_credentials()?;
            show_job(&creds, &project, job, status, follow, tail).await?;
        }
        Command::ListProjects {} => {
            let creds = load_credentials()?;
            list_projects(&creds).await?;
        }
        Command::ListPipelines {} => {
            let creds = load_credentials()?;
            list_pipelines(&creds, &project).await?;
        }
    }

    Ok(())
}
