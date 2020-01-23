use super::models::*;
use super::USER_AGENT;
use log::debug;

type BoxError = Box<dyn std::error::Error>;

pub struct Client {
    client: reqwest::Client,
    base_url: String,
}

impl Client {
    pub fn new(base_url: String, username: String, password: String) -> Result<Client, BoxError> {
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
        let raw_url = format!("{base}/api/admin/users", base = &self.base_url);
        let url = reqwest::Url::parse(&raw_url)?;
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

    pub async fn lookup_user(&self, login_or_email: &str) -> Result<User, BoxError> {
        let raw_url = format!("{base}/api/users/lookup", base = &self.base_url);
        let url = reqwest::Url::parse_with_params(&raw_url, &[("loginOrEmail", login_or_email)])?;
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

    pub async fn create_organization(
        &self,
        org: CreateOrganization,
    ) -> Result<CreateOrganizationResponse, BoxError> {
        let raw_url = format!("{base}/api/orgs", base = &self.base_url);
        let url = reqwest::Url::parse(&raw_url)?;
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

    pub async fn lookup_organization(&self, name: &str) -> Result<Organization, BoxError> {
        let raw_url = format!(
            "{base}/api/orgs/name/{org_name}",
            base = &self.base_url,
            org_name = name
        );
        let url = reqwest::Url::parse(&raw_url)?;
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

    pub async fn add_user_to_organization(
        &self,
        org_id: i32,
        user: CreateOrganizationMembership,
    ) -> Result<GenericResponse, BoxError> {
        let raw_url = format!(
            "{base}/api/orgs/{org_id}/users",
            base = &self.base_url,
            org_id = org_id
        );
        let url = reqwest::Url::parse(&raw_url)?;
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
        let raw_url = format!(
            "{base}/api/orgs/{org_id}/users/{user_id}",
            base = &self.base_url,
            org_id = org_id,
            user_id = user_id,
        );
        let url = reqwest::Url::parse(&raw_url)?;
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
        let raw_url = format!(
            "{base}/api/users/{user_id}/orgs",
            base = &self.base_url,
            user_id = user_id,
        );
        let url = reqwest::Url::parse(&raw_url)?;
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
}
