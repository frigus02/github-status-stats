use super::models::*;
use super::USER_AGENT;
use log::debug;
use reqwest::StatusCode;

type BoxError = Box<dyn std::error::Error>;

pub struct Client {
    client: reqwest::Client,
    base_url: String,
}

impl Client {
    pub fn new(base_url: String, username: &str, password: &str) -> Result<Client, BoxError> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static(USER_AGENT),
        );
        let auth = format!("{}:{}", username, password);
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Basic {}", base64::encode(&auth)).parse()?,
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Client { client, base_url })
    }

    pub async fn create_user(&self, user: CreateUser) -> Result<CreateUserResponse, BoxError> {
        let mut url = reqwest::Url::parse(&self.base_url)?;
        url.path_segments_mut()
            .map_err(|_| "cannot be base")?
            .extend(&["api", "admin", "users"]);
        debug!("request POST {} with body {:?}", url, user);
        let res = self
            .client
            .post(url)
            .json(&user)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }

    pub async fn lookup_user(&self, login_or_email: &str) -> Result<Option<User>, BoxError> {
        let mut url = reqwest::Url::parse(&self.base_url)?;
        url.path_segments_mut()
            .map_err(|_| "cannot be base")?
            .extend(&["api", "users", "lookup"]);
        url.query_pairs_mut()
            .append_pair("loginOrEmail", login_or_email);
        debug!("request GET {}", url);
        let res = self.client.get(url).send().await?;
        if res.status() == StatusCode::NOT_FOUND {
            Ok(None)
        } else {
            Ok(Some(res.error_for_status()?.json().await?))
        }
    }

    pub async fn create_organization(
        &self,
        org: CreateOrganization,
    ) -> Result<CreateOrganizationResponse, BoxError> {
        let mut url = reqwest::Url::parse(&self.base_url)?;
        url.path_segments_mut()
            .map_err(|_| "cannot be base")?
            .extend(&["api", "orgs"]);
        debug!("request POST {} with body {:?}", url, org);
        let res = self
            .client
            .post(url)
            .json(&org)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }

    pub async fn lookup_organization(&self, name: &str) -> Result<Option<Organization>, BoxError> {
        let mut url = reqwest::Url::parse(&self.base_url)?;
        url.path_segments_mut()
            .map_err(|_| "cannot be base")?
            .extend(&["api", "orgs", "name", name]);
        debug!("request GET {}", url);
        let res = self.client.get(url).send().await?;
        if res.status() == StatusCode::NOT_FOUND {
            Ok(None)
        } else {
            Ok(Some(res.error_for_status()?.json().await?))
        }
    }

    pub async fn add_user_to_organization(
        &self,
        org_id: i32,
        user: CreateOrganizationMembership,
    ) -> Result<GenericResponse, BoxError> {
        let mut url = reqwest::Url::parse(&self.base_url)?;
        url.path_segments_mut()
            .map_err(|_| "cannot be base")?
            .extend(&["api", "orgs", &org_id.to_string(), "users"]);
        debug!("request POST {} with body {:?}", url, user);
        let res = self
            .client
            .post(url)
            .json(&user)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }

    pub async fn remove_user_from_organization(
        &self,
        org_id: i32,
        user_id: i32,
    ) -> Result<GenericResponse, BoxError> {
        let mut url = reqwest::Url::parse(&self.base_url)?;
        url.path_segments_mut()
            .map_err(|_| "cannot be base")?
            .extend(&[
                "api",
                "orgs",
                &org_id.to_string(),
                "users",
                &user_id.to_string(),
            ]);
        debug!("request DELETE {}", url);
        let res = self
            .client
            .delete(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }

    pub async fn get_organizations_for_user(
        &self,
        user_id: i32,
    ) -> Result<Vec<OrganizationMembership>, BoxError> {
        let mut url = reqwest::Url::parse(&self.base_url)?;
        url.path_segments_mut()
            .map_err(|_| "cannot be base")?
            .extend(&["api", "users", &user_id.to_string(), "orgs"]);
        debug!("request GET {}", url);
        let res = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }

    pub async fn switch_organization_context(
        &self,
        org_id: i32,
    ) -> Result<GenericResponse, BoxError> {
        let mut url = reqwest::Url::parse(&self.base_url)?;
        url.path_segments_mut()
            .map_err(|_| "cannot be base")?
            .extend(&["api", "user", "using", &org_id.to_string()]);
        debug!("request POST {}", url);
        let res = self
            .client
            .post(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }

    pub async fn create_datasource(
        &self,
        datasource: CreateDataSource,
    ) -> Result<CreateDataSourceResponse, BoxError> {
        let mut url = reqwest::Url::parse(&self.base_url)?;
        url.path_segments_mut()
            .map_err(|_| "cannot be base")?
            .extend(&["api", "datasources"]);
        debug!("request POST {} with body {:?}", url, datasource);
        let res = self
            .client
            .post(url)
            .json(&datasource)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }

    pub async fn lookup_datasource(&self, name: &str) -> Result<Option<DataSource>, BoxError> {
        let mut url = reqwest::Url::parse(&self.base_url)?;
        url.path_segments_mut()
            .map_err(|_| "cannot be base")?
            .extend(&["api", "datasources", "name", name]);
        debug!("request GET {}", url);
        let res = self.client.get(url).send().await?;
        if res.status() == StatusCode::NOT_FOUND {
            Ok(None)
        } else {
            Ok(Some(res.error_for_status()?.json().await?))
        }
    }

    pub async fn update_datasource(
        &self,
        id: i32,
        datasource: UpdateDataSource,
    ) -> Result<UpdateDataSourceResponse, BoxError> {
        let mut url = reqwest::Url::parse(&self.base_url)?;
        url.path_segments_mut()
            .map_err(|_| "cannot be base")?
            .extend(&["api", "datasources", &id.to_string()]);
        debug!("request PUT {} with body {:?}", url, datasource);
        let res = self
            .client
            .put(url)
            .json(&datasource)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }

    pub async fn create_or_update_dashboard(
        &self,
        dashboard: CreateOrUpdateDashboard,
    ) -> Result<CreateOrUpdateDashboardResponse, BoxError> {
        let mut url = reqwest::Url::parse(&self.base_url)?;
        url.path_segments_mut()
            .map_err(|_| "cannot be base")?
            .extend(&["api", "dashboards", "db"]);
        debug!("request POST {} with body {:?}", url, dashboard);
        let res = self
            .client
            .post(url)
            .json(&dashboard)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }
}
