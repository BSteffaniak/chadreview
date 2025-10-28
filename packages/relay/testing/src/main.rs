use chadreview_relay_models::{CommentAction, PrAction};
use chadreview_relay_testing::{WebhookBuilder, WebhookSender};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "chadreview-relay-test")]
#[command(about = "Send mock GitHub webhook events to a ChadReview relay server", long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "http://localhost:8080")]
    url: String,

    #[arg(short, long, default_value = "test-instance")]
    instance_id: String,

    #[arg(short, long)]
    secret: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Send an issue comment event")]
    IssueComment {
        #[arg(long)]
        owner: String,

        #[arg(long)]
        repo: String,

        #[arg(long)]
        pr: u64,

        #[arg(long, default_value = "created")]
        action: String,

        #[arg(long)]
        body: String,

        #[arg(long)]
        user: Option<String>,

        #[arg(long)]
        user_id: Option<u64>,
    },
    #[command(about = "Send a pull request review comment event")]
    ReviewComment {
        #[arg(long)]
        owner: String,

        #[arg(long)]
        repo: String,

        #[arg(long)]
        pr: u64,

        #[arg(long, default_value = "created")]
        action: String,

        #[arg(long)]
        body: String,

        #[arg(long)]
        path: String,

        #[arg(long)]
        line: u64,

        #[arg(long)]
        user: Option<String>,

        #[arg(long)]
        user_id: Option<u64>,
    },
    #[command(about = "Send a pull request event")]
    PullRequest {
        #[arg(long)]
        owner: String,

        #[arg(long)]
        repo: String,

        #[arg(long)]
        pr: u64,

        #[arg(long, default_value = "opened")]
        action: String,

        #[arg(long)]
        user: Option<String>,

        #[arg(long)]
        user_id: Option<u64>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let sender = WebhookSender::new(&cli.url);

    match cli.command {
        Command::IssueComment {
            owner,
            repo,
            pr,
            action,
            body,
            user,
            user_id,
        } => {
            let action = parse_comment_action(&action)?;
            let mut builder = WebhookBuilder::new(&owner, &repo, pr);

            if let (Some(user), Some(user_id)) = (user, user_id) {
                builder = builder.with_user(&user, user_id);
            }

            let payload = builder.build_issue_comment(action, &body);

            println!(
                "Sending issue_comment event to {}/webhook/{}",
                cli.url, cli.instance_id
            );
            println!("Payload: {}", serde_json::to_string_pretty(&payload)?);

            let response = sender
                .send_webhook(
                    &cli.instance_id,
                    "issue_comment",
                    payload,
                    cli.secret.as_deref(),
                )
                .await?;

            println!("\n✓ Success! Server responded with: {}", response.status());
        }
        Command::ReviewComment {
            owner,
            repo,
            pr,
            action,
            body,
            path,
            line,
            user,
            user_id,
        } => {
            let action = parse_comment_action(&action)?;
            let mut builder = WebhookBuilder::new(&owner, &repo, pr);

            if let (Some(user), Some(user_id)) = (user, user_id) {
                builder = builder.with_user(&user, user_id);
            }

            let payload = builder.build_review_comment(action, &body, &path, line);

            println!(
                "Sending pull_request_review_comment event to {}/webhook/{}",
                cli.url, cli.instance_id
            );
            println!("Payload: {}", serde_json::to_string_pretty(&payload)?);

            let response = sender
                .send_webhook(
                    &cli.instance_id,
                    "pull_request_review_comment",
                    payload,
                    cli.secret.as_deref(),
                )
                .await?;

            println!("\n✓ Success! Server responded with: {}", response.status());
        }
        Command::PullRequest {
            owner,
            repo,
            pr,
            action,
            user,
            user_id,
        } => {
            let action = parse_pr_action(&action)?;
            let mut builder = WebhookBuilder::new(&owner, &repo, pr);

            if let (Some(user), Some(user_id)) = (user, user_id) {
                builder = builder.with_user(&user, user_id);
            }

            let payload = builder.build_pull_request(action);

            println!(
                "Sending pull_request event to {}/webhook/{}",
                cli.url, cli.instance_id
            );
            println!("Payload: {}", serde_json::to_string_pretty(&payload)?);

            let response = sender
                .send_webhook(
                    &cli.instance_id,
                    "pull_request",
                    payload,
                    cli.secret.as_deref(),
                )
                .await?;

            println!("\n✓ Success! Server responded with: {}", response.status());
        }
    }

    Ok(())
}

fn parse_comment_action(action: &str) -> anyhow::Result<CommentAction> {
    match action.to_lowercase().as_str() {
        "created" => Ok(CommentAction::Created),
        "edited" => Ok(CommentAction::Edited),
        "deleted" => Ok(CommentAction::Deleted),
        _ => anyhow::bail!(
            "Invalid comment action: {action}. Must be one of: created, edited, deleted"
        ),
    }
}

fn parse_pr_action(action: &str) -> anyhow::Result<PrAction> {
    match action.to_lowercase().as_str() {
        "opened" => Ok(PrAction::Opened),
        "edited" => Ok(PrAction::Edited),
        "closed" => Ok(PrAction::Closed),
        "reopened" => Ok(PrAction::Reopened),
        "synchronize" => Ok(PrAction::Synchronize),
        _ => anyhow::bail!(
            "Invalid PR action: {action}. Must be one of: opened, edited, closed, reopened, synchronize"
        ),
    }
}
