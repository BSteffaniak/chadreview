use chadreview_pr_models::{PrState, PullRequest};
use hyperchad::router::Container;
use hyperchad::template::container;
use hyperchad_template::LayoutOverflow;

#[must_use]
pub fn render_pr_header(pr: &PullRequest) -> Container {
    let draft_badge = render_draft_badge(pr.draft);
    let labels_section = render_labels(&pr.labels);
    let people_section = render_people(&pr.assignees, &pr.reviewers);

    container! {
        header padding=20 {
            (render_header_main(pr, &draft_badge))
            (render_metadata(pr))
            (labels_section)
            (people_section)
            (render_description(&pr.description))
        }
    }
    .into()
}

fn render_header_main(pr: &PullRequest, draft_badge: &Container) -> Container {
    let (state_text, state_bg, state_color) = match pr.state {
        PrState::Open => ("Open", "#1a7f37", "#ffffff"),
        PrState::Closed => ("Closed", "#cf222e", "#ffffff"),
        PrState::Merged => ("Merged", "#8250df", "#ffffff"),
    };

    container! {
        div margin-bottom=16 {
            div direction=row align-items=center gap=12 margin-bottom=12 {
                h1 font-size=32 font-weight=600 color="#24292f" margin=0 {
                    (pr.title)
                }
                span font-size=32 font-weight=300 color="#57606a" {
                    "#" (pr.number)
                }
            }
            div direction=row align-items=center gap=8 {
                span
                    padding-y=4
                    padding-x=12
                    border-radius=20
                    background=(state_bg)
                    color=(state_color)
                    font-size=14
                    font-weight=500
                {
                    (state_text)
                }
                (draft_badge)
            }
        }
    }
    .into()
}

fn render_draft_badge(draft: bool) -> Container {
    if draft {
        container! {
            span
                padding-y=4
                padding-x=12
                border-radius=20
                background="#6e7781"
                color="#ffffff"
                font-size=14
                font-weight=500
            {
                "Draft"
            }
        }
    } else {
        container! { div {} }
    }
    .into()
}

fn render_metadata(pr: &PullRequest) -> Container {
    container! {
        div border-top="1px solid #d0d7de" padding-top=16 margin-bottom=16 {
            div
                direction=row
                gap=24
                margin-bottom=12
                overflow-x=(LayoutOverflow::Wrap { grid: false })
            {
                div direction=row align-items=center gap=8 {
                    span color="#57606a" font-weight=600 { "Author:" }
                    image src=(pr.author.avatar_url) width=32 height=32 border-radius=16 {}
                    anchor href=(pr.author.html_url) color="#0969da" font-weight=600 {
                        (pr.author.username)
                    }
                }
                div
                    direction=row
                    align-items=center
                    gap=8
                {
                    span color="#57606a" font-weight=600 { "Branch:" }
                    span
                        font-family="monospace"
                        font-size=13
                        padding-y=2
                        padding-x=6
                        background="#eff2f5"
                        border-radius=6
                        color="#24292f"
                    {
                        (pr.head_branch)
                    }
                    span color="#57606a" { "→" }
                    span
                        font-family="monospace"
                        font-size=13
                        padding-y=2
                        padding-x=6
                        background="#eff2f5"
                        border-radius=6
                        color="#24292f"
                    {
                        (pr.base_branch)
                    }
                }
            }
            div direction=row gap=24 color="#57606a" font-size=13 {
                span { "Created: " (pr.created_at.to_rfc3339()) }
                span { "Updated: " (pr.updated_at.to_rfc3339()) }
            }
        }
    }
    .into()
}

fn render_labels(labels: &[chadreview_pr_models::Label]) -> Container {
    if labels.is_empty() {
        return container! { div {} }.into();
    }

    container! {
        div direction=row align-items=center gap=8 margin-bottom=16 {
            span color="#57606a" font-weight=600 { "Labels:" }
            @for label in labels {
                span
                    padding-y=4
                    padding-x=10
                    border-radius=12
                    font-size=12
                    font-weight=500
                    background=(format!("#{}", label.color))
                    color="#ffffff"
                {
                    (label.name)
                }
            }
        }
    }
    .into()
}

fn render_people(
    assignees: &[chadreview_pr_models::User],
    reviewers: &[chadreview_pr_models::User],
) -> Container {
    if assignees.is_empty() && reviewers.is_empty() {
        return container! { div {} }.into();
    }

    let assignees_section = render_assignees(assignees);
    let reviewers_section = render_reviewers(reviewers);

    container! {
        div direction=row gap=24 margin-bottom=16 {
            (assignees_section)
            (reviewers_section)
        }
    }
    .into()
}

fn render_assignees(assignees: &[chadreview_pr_models::User]) -> Container {
    if assignees.is_empty() {
        return container! { div {} }.into();
    }

    container! {
        div direction=row align-items=center gap=8 {
            span color="#57606a" font-weight=600 { "Assignees:" }
            @for assignee in assignees {
                div direction=row align-items=center gap=4 {
                    image src=(assignee.avatar_url) width=24 height=24 border-radius=12 {}
                    anchor href=(assignee.html_url) color="#0969da" font-weight=600 {
                        (assignee.username)
                    }
                }
            }
        }
    }
    .into()
}

fn render_reviewers(reviewers: &[chadreview_pr_models::User]) -> Container {
    if reviewers.is_empty() {
        return container! { div {} }.into();
    }

    container! {
        div direction=row align-items=center gap=8 {
            span color="#57606a" font-weight=600 { "Reviewers:" }
            @for reviewer in reviewers {
                div direction=row align-items=center gap=4 {
                    image src=(reviewer.avatar_url) width=24 height=24 border-radius=12 {}
                    anchor href=(reviewer.html_url) color="#0969da" font-weight=600 {
                        (reviewer.username)
                    }
                }
            }
        }
    }
    .into()
}

fn render_description(description: &str) -> Container {
    container! {
        section margin-top=20 padding-top=20 border-top="1px solid #d0d7de" {
            h3 font-size=16 font-weight=600 color="#24292f" margin-bottom=12 {
                "Description"
            }
            div color="#24292f" {
                (description)
            }
        }
    }
    .into()
}
