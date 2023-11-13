use std::sync::Arc;

use anyhow::Result;
use prettytable::{format, row, table, Table};
use reqwest::Url;

use crate::credentials::Credentials;
use crate::format::{format_bytes, format_seconds};
use crate::job::{find_jobs, get_job_details, Job};
use crate::ShowJobArgs;

pub async fn show_job(
    creds: &Credentials,
    project: &str,
    args: &ShowJobArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let jobs: Vec<Job> = if let Some(pipeline) = args.pipeline {
        find_jobs(creds, project, Some(vec![pipeline]), None, None, None).await?
    } else {
        vec![
            get_job_details(
                Arc::new(creds.clone()),
                project.to_string(),
                args.job.unwrap(),
            )
            .await?,
        ]
    };

    let creds = Arc::new(creds.clone());
    let project = project.to_string();

    for job in jobs.iter() {
        let log = get_job_logs(Arc::clone(&creds), project.clone(), job.id).await?;
        let log = if args.plain {
            strip_ansi_escapes::strip_str(log)
        } else {
            log
        };
        let skip = match args.tail {
            Some(t) => log.lines().count().saturating_sub(t),
            _ => 0,
        };
        for l in log.lines().skip(skip) {
            let l = if args.prefix {
                format!("{}: {}", job.name, l)
            } else {
                l.to_string()
            };
            println!("{}", l);
        }
    }

    /*
    let logs: Result<Vec<String>, anyhow::Error> = async {
        let futures: Vec<_> = jobs
            .iter()
            .map(|job| {
                println!("getting {:#?}", job);
                get_job_logs(Arc::clone(&creds), project.clone(), job.id)
            })
            .collect();

        let mut results = Vec::with_capacity(futures.len());

        for future in futures {
            let log = future.await?;
            if let Some(tail) = tail {
                let skip: usize = log.lines().count().saturating_sub(tail);
                let lines: Vec<&str> = log.lines().skip(skip).collect();
                println!("\nJob Logs (last {} lines):", tail);
                println!("{}", lines.join("\n"));
            } else {
                println!("\nJob Logs:");
                println!("{}", log);
            }
        }
    }.await;
    */

    // Now job_details and job_logs are available, you can print them or process further

    if args.status {
        for job in &jobs {
            let artifact_size = job.artifacts.iter().map(|a| a.size).sum();
            let mut artifact_table = job
                .artifacts
                .iter()
                .map(|a| row!(a.filename, format_bytes(a.size)))
                .collect::<Table>();
            artifact_table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
            artifact_table.set_titles(row!("filename", "size"));

            let mut table = table!(
                ["ID", job.id],
                ["Status", job.status],
                ["Stage", job.stage],
                ["Name", job.name],
                ["Artifact size", format_bytes(artifact_size)],
                ["Artifacts", artifact_table],
                ["Started at", job.started_at],
                ["Finished at", job.finished_at],
                [
                    "Duration",
                    &format_seconds(job.duration.unwrap_or_default())
                ],
                ["Runner", job.runner.as_ref().unwrap().description],
                ["", ""],
                ["Ref", job.pipeline.rref],
                ["Source", job.pipeline.source],
                ["Pipeline URL", job.pipeline.web_url]
            );

            table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
            table.printstd();
        }
    }

    Ok(())
}

async fn get_job_logs(creds: Arc<Credentials>, project: String, job: usize) -> Result<String> {
    let url = format!(
        "{}/api/v4/projects/{}/jobs/{}/trace",
        creds.url, project, job
    );
    let url = Url::parse(&url)?;

    let client = reqwest::Client::new();
    let response = client.get(url).bearer_auth(&creds.token).send().await?;

    let logs = response.text().await?;

    Ok(logs)
}
