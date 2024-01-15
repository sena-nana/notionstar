use dotenv::dotenv;
use indicatif::ProgressBar;
use notion::{
    ids::PropertyId,
    models::{
        properties::{DateOrDateTime, DateValue, PropertyValue},
        text::{Annotations, RichText, RichTextCommon, Text},
        DateTime, Page, PageCreateRequest, Parent, Properties, Utc,
    },
    NotionApi,
};
use octocrab::{
    self,
    models::{repos::Release, Repository},
};
use std::env;
use std::{collections::HashMap, str::FromStr};
use tokio;
#[tokio::main]
async fn main() {
    dotenv().ok();
    let notion = Notion::new().await;
    // let database = notion.get_database().await;
    let stars = notion.get_stars().await;
    let pb = ProgressBar::new(stars.len() as u64);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{msg} {bar}")
            .unwrap(),
    );
    for star in stars {
        pb.set_message("adding ".to_string() + &star.name);
        notion.add_repo(star).await;
        pb.inc(1);
    }
}

struct Notion {
    api: NotionApi,
    database_id: notion::ids::DatabaseId,
    github: octocrab::Octocrab,
}
impl Notion {
    async fn new() -> Notion {
        Notion {
            api: NotionApi::new(env::var("NOTION_API").unwrap()).unwrap(),
            database_id: notion::ids::DatabaseId::from_str(env::var("DATABASE").unwrap().as_str())
                .unwrap(),
            github: octocrab::Octocrab::builder()
                .personal_token(env::var("GITHUB_API").unwrap())
                .build()
                .unwrap(),
        }
    }
    async fn get_database(&self) -> Vec<notion::models::Page> {
        let database = self
            .api
            .query_database(
                &self.database_id,
                notion::models::search::DatabaseQuery::default(),
            )
            .await
            .unwrap();
        return database.results;
    }

    async fn add_repo(&self, stars: Repository) -> Page {
        let owner = stars.owner.unwrap().login;
        let name = stars.name;
        let release = self.get_release(&owner, &name).await;
        let lastupdate = match release {
            Ok(release) => Some(release.published_at.unwrap()),
            Err(_) => None,
        };
        return self
            .new_data(
                name.to_owned(),
                stars.html_url.unwrap().to_string(),
                owner.to_owned(),
                lastupdate,
            )
            .await;
    }
    async fn new_data(
        &self,
        name: String,
        release: String,
        owner: String,
        lastupdate: Option<DateTime<Utc>>,
    ) -> Page {
        let properties = Properties {
            properties: HashMap::from([
                (
                    "名称".to_string(),
                    PropertyValue::Title {
                        id: PropertyId::from_str("title").unwrap(),
                        title: text(name),
                    },
                ),
                (
                    "release".to_owned(),
                    PropertyValue::Url {
                        id: PropertyId::from_str("pr%7Cj").unwrap(),
                        url: Some(release),
                    },
                ),
                (
                    "上次release".to_owned(),
                    PropertyValue::Date {
                        id: PropertyId::from_str("pvki").unwrap(),
                        date: match lastupdate {
                            Some(_) => Some(DateValue {
                                start: DateOrDateTime::Date(lastupdate.unwrap().date_naive()),
                                end: None,
                                time_zone: None,
                            }),
                            None => None,
                        },
                    },
                ),
                (
                    "owner".to_owned(),
                    PropertyValue::Text {
                        id: PropertyId::from_str("OHG%3B").unwrap(),
                        rich_text: text(owner),
                    },
                ),
            ]),
        };

        return self
            .api
            .create_page(PageCreateRequest {
                parent: Parent::Database {
                    database_id: self.database_id.to_owned(),
                },
                properties: properties,
            })
            .await
            .unwrap();
    }
    async fn get_stars(&self) -> Vec<octocrab::models::Repository> {
        let mut stars = Vec::new();
        let mut page = 1;
        loop {
            let star_page = self
                .github
                .current()
                .list_repos_starred_by_authenticated_user()
                .per_page(100)
                .page(page)
                .send()
                .await
                .unwrap()
                .items;

            if (&star_page).is_empty() {
                break;
            }
            stars.extend(star_page);
            page += 1;
            println!("stars count {}", stars.len());
        }
        println!("stars getting finished");
        return stars;
    }
    async fn get_release(&self, owner: &String, name: &String) -> Result<Release, octocrab::Error> {
        return self.github.repos(owner, name).releases().get_latest().await;
    }
}

fn text(name: String) -> Vec<RichText> {
    Vec::from([RichText::Text {
        rich_text: RichTextCommon {
            plain_text: name.to_owned(),
            href: None,
            annotations: Some(Annotations {
                bold: Some(false),
                code: Some(false),
                color: Some(notion::models::text::TextColor::Default),
                italic: Some(false),
                strikethrough: Some(false),
                underline: Some(false),
            }),
        },
        text: Text {
            content: name,
            link: None,
        },
    }])
}
