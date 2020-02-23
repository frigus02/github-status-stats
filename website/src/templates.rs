use handlebars::Handlebars;
use serde::Serialize;
use std::fs;

lazy_static! {
    static ref HANDLEBARS: Handlebars<'static> = {
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

        hb
    };
}

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
    },
}

pub fn render_index(data: &IndexTemplate) -> String {
    HANDLEBARS
        .render("index.html", data)
        .unwrap_or_else(|err| err.to_string())
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

pub fn render_dashboard(data: &DashboardTemplate) -> String {
    HANDLEBARS
        .render("dashboard.html", data)
        .unwrap_or_else(|err| err.to_string())
}
