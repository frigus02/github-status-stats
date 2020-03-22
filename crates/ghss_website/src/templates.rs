use handlebars::Handlebars;
use serde::Serialize;
use std::convert::Infallible;
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
            .render("index", data)
            .unwrap_or_else(|err| err.to_string())
    }

    pub fn render_dashboard(&self, data: &DashboardTemplate) -> String {
        self.hb
            .render("dashboard", data)
            .unwrap_or_else(|err| err.to_string())
    }
}

pub fn load() -> Templates<'static> {
    let mut hb = Handlebars::new();

    hb.register_template_file("layout", "templates/_layout.handlebars")
        .expect("register layout");
    hb.register_template_file("index", "templates/index.handlebars")
        .expect("register index");
    hb.register_template_file("dashboard", "templates/dashboard.handlebars")
        .expect("register dashboard");

    Templates { hb }
}

pub fn with_templates(
    config: Arc<Templates<'static>>,
) -> impl Filter<Extract = (Arc<Templates<'static>>,), Error = Infallible> + Clone {
    warp::any().map(move || config.clone())
}
