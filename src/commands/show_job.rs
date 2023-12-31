use std::collections::HashMap;
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
        find_jobs(creds, project, vec![pipeline], None, None, None).await?
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

        if args.stats {
            let mut starts = HashMap::new();

            let mut level = 0;

            let mut table = Table::new();
            table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
            let titles = row!["Section", "Time", "Command"];
            table.set_titles(titles);

            let mut cursteps: Vec<_> = Vec::new();
            let startidx: Vec<_> = log.match_indices("section_start:").collect();
            let endidx: Vec<_> = log.match_indices("section_end:").collect();

            for (idx, _) in itertools::merge(startidx, endidx) {
                let sec = &log[idx..];
                let mut f = sec.split(&[':', '\r', '\n'][..]);
                match f.next() {
                    Some("section_start") => {
                        let stime = f.next().unwrap();
                        let stime = stime.parse::<usize>().unwrap();
                        let sstep = f.next().unwrap();
                        let scmd = f.next().unwrap_or("none");
                        let scmd = strip_ansi_escapes::strip_str(scmd);
                        starts.insert(sstep.to_string(), (stime, scmd.to_string()));
                        cursteps.push(sstep);
                        level += 1;
                    }
                    Some("section_end") => {
                        level -= 1;
                        cursteps.pop();
                        let stime = f.next().unwrap();
                        let stime = stime.parse::<usize>().unwrap();
                        let sstep = f.next().unwrap();
                        table.add_row(row![
                            "  ".repeat(level) + sstep,
                            r->format_seconds((stime - starts[sstep].0) as f64),
                            starts[sstep].1
                        ]);
                    }
                    None => {
                        while let Some(sstep) = cursteps.pop() {
                            table.add_row(row![
                                "  ".repeat(level) + sstep,
                                "(running)",
                                starts[sstep].1
                            ]);
                            level -= 1;
                        }
                    }
                    Some(s) => {
                        println!("Unknown section {}", s);
                    }
                };
            }
            table.printstd();
        } else {
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
            let mut artifact_table = match &job.artifacts {
                Some(a) => a
                    .iter()
                    .map(|a| row!(a.filename, format_bytes(a.size)))
                    .collect::<Table>(),
                None => table![],
            };
            artifact_table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
            artifact_table.set_titles(row!("filename", "size"));

            let mut table = table!(
                ["ID", job.id],
                ["Status", job.status],
                ["Stage", job.stage],
                ["Name", job.name],
                ["Artifact size", format_bytes(job.artifacts_size)],
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
