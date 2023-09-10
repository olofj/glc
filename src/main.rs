use parse_duration::parse;
use structopt::StructOpt;

mod commands {
    pub mod credentials;
    pub mod get_artifact;
    pub mod job;
    pub mod job_history;
    pub mod list_jobs;
    pub mod list_pipelines;
    pub mod list_projects;
    pub mod login;
    pub mod pipeline;
    pub mod runner;
    pub mod show_job;
}
mod format;

use commands::credentials::load_credentials;
use commands::get_artifact::get_artifact;
use commands::job_history::job_history;
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
        #[structopt(short = "p", long = "pipelines")]
        pipelines: Option<Vec<usize>>,
        /// Max history ("1h", "10m", "4d" etc)
        #[structopt(short = "m", long = "max-age")]
        max_age: Option<String>,
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
    /// Get artifact from job
    #[structopt(name = "get-artifact")]
    GetArtifact {
        /// Job ID to download from
        #[structopt(short = "j", long = "job")]
        job: usize,
        /// Artifact name
        #[structopt(short = "n", long = "name")]
        name: String,
    },
    /// Show historical results for a job (by name)
    #[structopt(name = "job-history")]
    JobHistory {
        /// Job name
        #[structopt(short = "n", long = "name")]
        name: String,
        /// Max history ("1h", "10m", "4d" etc)
        #[structopt(short = "m", long = "max-age")]
        max_age: Option<String>,
        /// Source (type of pipeline)
        #[structopt(short = "s", long = "source")]
        source: Option<String>,
        /// Reference (branch)
        #[structopt(short = "r", long = "ref")]
        rref: Option<String>,
    },
    /// List pipelines
    #[structopt(name = "list-pipelines")]
    ListPipelines {
        /// Max history ("1h", "10m", "4d" etc)
        #[structopt(short = "m", long = "max-age")]
        max_age: Option<String>,
        /// Source (type of pipeline)
        #[structopt(short = "s", long = "source")]
        source: Option<String>,
        /// Reference (branch)
        #[structopt(short = "r", long = "ref")]
        rref: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    let project = opt.project.unwrap_or_default();
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
    let creds = load_credentials()?;

    match opt.cmd {
        Command::Login { token, url } => {
            login(&token, &url)?;
        }
        Command::ListJobs { pipelines, max_age } => {
            let max_age = match max_age {
                None => None,
                Some(a) => parse(&a).ok(),
            }
            .map(|a| a.as_secs() as isize);
            list_jobs(&creds, &project, pipelines, max_age).await?;
        }
        Command::ShowJob {
            job,
            status,
            follow,
            tail,
        } => {
            show_job(&creds, &project, job, status, follow, tail).await?;
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
            let max_age = match max_age {
                None => None,
                Some(a) => parse(&a).ok(),
            }
            .map(|a| a.as_secs() as isize);
            job_history(&creds, &project, &name, max_age, source, rref).await?;
        }
        Command::ListProjects {} => {
            list_projects(&creds).await?;
        }
        Command::ListPipelines {
            max_age,
            source,
            rref,
        } => {
            let max_age = match max_age {
                None => None,
                Some(a) => parse(&a).ok(),
            }
            .map(|a| a.as_secs() as isize);
            list_pipelines(&creds, &project, max_age, source, rref).await?;
        }
    }

    Ok(())
}
