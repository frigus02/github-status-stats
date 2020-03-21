use handlebars::Handlebars;
use serde::Serialize;
use std::convert::Infallible;
use std::fs;
use std::sync::Arc;
use warp::Filter;

#[derive(Serialize)]
pub struct RepositoryAccess {
    pub name: String,
}

#[derive(Serialize)]
pub enum IndexTemplate {
    Anonymous {
        login_url: String,
    },
    LoggedIn {
        user: String,
        repositories: Vec<RepositoryAccess>,
        login_url: String,
    },
}

#[derive(Serialize)]
pub enum DashboardData {
    Data { repository_id: i32 },
    Error { message: String },
}

#[derive(Serialize)]
pub struct DashboardTemplate {
    pub user: String,
    pub repository_name: String,
    pub data: DashboardData,
}

pub struct Templates<'a> {
    hb: Handlebars<'a>,
}

impl<'a> Templates<'a> {
    pub fn render_index(&self, data: &IndexTemplate) -> String {
        self.hb
            .render("index.html", data)
            .unwrap_or_else(|err| err.to_string())
    }

    pub fn render_dashboard(&self, data: &DashboardTemplate) -> String {
        self.hb
            .render("dashboard.html", data)
            .unwrap_or_else(|err| err.to_string())
    }
}

pub fn load() -> Templates<'static> {
    let mut hb = Handlebars::new();

    let index_template: String =
        String::from_utf8_lossy(&fs::read("templates/index.html").unwrap())
            .parse()
            .unwrap();
    hb.register_template_string("index.html", index_template)
        .expect("register index.html");

    let dashboard_template: String =
        String::from_utf8_lossy(&fs::read("templates/dashboard.html").unwrap())
            .parse()
            .unwrap();
    hb.register_template_string("dashboard.html", dashboard_template)
        .expect("register dashboard.html");

    Templates { hb }
}

pub fn with_templates(
    config: Arc<Templates<'static>>,
) -> impl Filter<Extract = (Arc<Templates<'static>>,), Error = Infallible> + Clone {
    warp::any().map(move || config.clone())
}
