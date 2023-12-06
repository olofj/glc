use clap::{ArgAction, Parser};
use parse_duration::parse;
use std::io::{self, Write};

mod commands {
    pub mod get_artifact;
    pub mod job_history;
    pub mod list_jobs;
    pub mod list_pipelines;
    pub mod list_projects;
    pub mod list_runners;
    pub mod login;
    pub mod show_job;
    pub mod test_report;
}
mod credentials;
mod format;
mod job;
mod pipeline;
mod project;
mod runner;

use commands::get_artifact::get_artifact;
use commands::job_history::job_history;
use commands::list_jobs::list_jobs;
use commands::list_pipelines::list_pipelines;
use commands::list_projects::list_projects;
use commands::list_runners::list_runners;
use commands::login::login;
use commands::show_job::show_job;
use credentials::load_credentials;

#[derive(Parser, Debug)]
#[clap(name = "glc", about = "gitlab client utility")]
struct Opt {
    /// The project ID
    #[clap(short = 'P', long = "project", default_value = "197")]
    project: String,

    #[command(subcommand)]
    cmd: Command,
}

#[derive(Parser, Debug)]
enum Command {
    /// Login to GitLab
    #[command(name = "login")]
    Login {
        /// GitLab URL
        #[clap(short = 'u', long = "url")]
        url: String,
    },

    /// List jobs
    #[command(name = "list-jobs")]
    ListJobs {
        /// Pipeline ID to list jobs for
        #[clap(short = 'p', long = "pipelines")]
        pipelines: Option<Vec<usize>>,
        /// Max history ("1h", "10m", "4d" etc)
        #[clap(short = 'm', default_value = "24h", long = "max-age")]
        max_age: String,
        /// Status ("Success", "Running", "Failed", etc)
        #[clap(short = 's', long = "status")]
        status: Option<String>,
    },

    /// List projects
    #[command(name = "list-projects")]
    ListProjects {},

    /// List runners
    #[command(name = "list-runners")]
    ListRunners {},

    /// Show job
    #[command(name = "show-job")]
    ShowJob(ShowJobArgs),

    /// Get artifact from job
    #[command(name = "get-artifact")]
    GetArtifact {
        /// Job ID to download from
        #[clap(short = 'j', long = "job")]
        job: usize,
        /// Artifact name
        #[clap(short = 'n', long = "name")]
        name: String,
    },

    /// Show historical results for a job (by name)
    #[command(name = "job-history")]
    JobHistory {
        /// Job name
        #[clap(short = 'n', long = "name")]
        name: String,
        /// Max history ("1h", "10m", "4d" etc)
        #[clap(short = 'm', default_value = "24h", long = "max-age")]
        max_age: String,
        /// Source (type of pipeline)
        #[clap(short = 's', long = "source")]
        source: Option<String>,
        /// Reference (branch)
        #[clap(short = 'r', long = "ref")]
        rref: Option<String>,
    },

    /// List pipelines
    #[command(name = "list-pipelines")]
    ListPipelines {
        /// Max history ("1h", "10m", "4d" etc)
        #[clap(short = 'm', default_value = "24h", long = "max-age")]
        max_age: String,
        /// Source (type of pipeline)
        #[clap(short = 's', long = "source")]
        source: Option<String>,
        /// Reference (branch)
        #[clap(short = 'r', long = "ref")]
        rref: Option<String>,
    },
}

#[derive(Parser, Debug)]
pub struct ShowJobArgs {
    /// The ID of the job to show
    #[clap(conflicts_with = "pipeline")]
    job: Option<usize>,
    /// Pipeline ID to show jobs for
    #[clap(short = 'p', long = "pipeline", conflicts_with = "job")]
    pipeline: Option<usize>,
    /// Status summary after output
    #[clap(long = "no-status", action = ArgAction::SetFalse)]
    status: bool,
    /// Follow (keep listening)
    #[clap(short = 'f', long = "follow", requires = "job")]
    _follow: Option<bool>,
    /// Number of lines of output to show (negative number)
    #[clap(short = 't', long = "tail")]
    tail: Option<usize>,
    /// Show job prefix for every line of log
    #[clap(long = "prefix")]
    prefix: bool,
    /// Remove all ANSI control characters
    #[clap(long = "plain")]
    plain: bool,
}

impl ShowJobArgs {
    fn validate(&mut self) -> Result<(), String> {
        if self.job.is_none() && self.pipeline.is_none() {
            return Err(String::from("Must specify either job or pipeline."));
        }
        if let Some(_) = self.pipeline {
            if self.status {
                self.status = false; // default for pipeline
            } else if self.status {
                return Err(String::from("Cannot use status with pipeline."));
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::parse();

    let project = opt.project;
    /*
    println!("repo path: {:#?}", project);
    let repo = gix::discover(project.clone())?;
    println!("repo: {:#?}", repo);
    let remote = repo.find_default_remote(gix::remote::Direction::Push);
    println!("remote: {:#?}", remote);
    */
    /*
    let project: String = if true {
        let repo_path = Path::new("."); // path to your git repository
        match Repository::discover(repo_path) {
            Ok(repo) => match repo.find_remote("origin") {
                Ok(remote) => remote.url().unwrap_or("No URL").to_string(),
                Err(e) => format!("Error: {}", e),
            },
            Err(e) => format!("Error: {}", e),
        }
    } else {
        opt.project.unwrap().to_string()
    };
    println!("Project: {:?}", project);
    */
    if let Command::Login { url } = opt.cmd {
        login(&url)?;
        return Ok(());
    }

    let creds = load_credentials()?;

    match opt.cmd {
        Command::Login { url } => {
            login(&url)?;
        }
        Command::ListJobs {
            pipelines,
            max_age,
            status,
        } => {
            let max_age = parse(&max_age)?.as_secs() as isize;
            let pipelines = pipelines.unwrap_or_else(Vec::new);
            println!("ListJobs max_age {:?}", max_age);
            list_jobs(&creds, &project, pipelines, max_age, status).await?;
        }
        Command::ShowJob(mut args) => {
            if let Err(err) = args.validate() {
                eprintln!("Error: {}", err);
                std::process::exit(1);
            }
            show_job(&creds, &project, &args).await?;
        }
        Command::GetArtifact { job, name } => {
            get_artifact(&creds, &project, job, name).await?;
        }
        Command::JobHistory {
            name,
            max_age,
            source,
            rref,
        } => {
            let max_age = parse(&max_age)?.as_secs() as isize;
            job_history(&creds, &project, &name, max_age, source, rref).await?;
        }
        Command::ListProjects {} => {
            list_projects(&creds).await?;
        }
        Command::ListRunners {} => {
            list_runners(&creds).await?;
        }
        Command::ListPipelines {
            max_age,
            source,
            rref,
        } => {
            let max_age = parse(&max_age)?.as_secs() as isize;
            list_pipelines(&creds, &project, max_age, source, rref).await?;
        }
    }

    io::stdout().flush().unwrap();
    Ok(())
}
