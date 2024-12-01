use crate::db;
use core::future::Future;
use sqlx::{FromRow, Sqlite};
use std::pin::Pin;
use std::sync::Arc;
use tokio;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

#[derive(Clone, FromRow, Debug)]
struct ServiceSchema {
    id: i64,
    name: String,
    enable: bool,
    creation_time: String,
}

pub struct Service {
    pub id: i64,
    pub name: String,
    pub enable: bool,
    pub creation_time: String,
    future_factory: Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>,
    join_handle: Option<JoinHandle<()>>,
}

impl Service {
    pub fn new(
        id: i64,
        name: String,
        enable: bool,
        creation_time: String,
        future_factory: Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>,
    ) -> Self {
        Self {
            id,
            name,
            enable,
            creation_time,
            future_factory,
            join_handle: None,
        }
    }

    pub fn begin(&mut self) -> &mut Self {
        if self.enable {
            let future = (self.future_factory)();
            log::info!("Beginning '{}' service..", &self.name);
            self.join_handle = Some(tokio::spawn(future));
        }
        self
    }

    pub fn end(&mut self) -> &mut Self {
        if let Some(join_handle) = self.join_handle.take() {
            log::info!("Ending '{}' service..", &self.name);
            join_handle.abort();
        }
        self
    }

    pub async fn set_enable_state(&mut self, state: bool) {
        if state == self.enable {
            return;
        }
        self.enable = state;
        if self.enable {
            self.begin();
        } else {
            self.end();
        }

        let db = db::get_db().await;
        let update_enable_state = sqlx::query("UPDATE services SET enable = False WHERE id = ?;")
            .bind(self.id)
            .execute(&db)
            .await;

        match update_enable_state {
            Ok(query_result) => {
                log::info!(
                    "Update of service enable state with name: {} was successful! {:?}",
                    &self.name,
                    query_result
                );
            }
            Err(err) => {
                log::error!(
                    "Update of service enable state with name: {} failed! {:?}",
                    &self.name,
                    err
                );
            }
        }
    }
}

pub struct Services {
    pub services: Vec<Arc<Mutex<Service>>>,
}

impl Default for Services {
    fn default() -> Self {
        Self::new()
    }
}

impl Services {
    pub fn new() -> Self {
        Self {
            services: Vec::new(),
        }
    }

    pub async fn create_service(
        &mut self,
        name: String,
        enable: bool,
        future_factory: Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>,
    ) {
        let service_index = self.get_service_internally(&name).await;

        if service_index.is_some() {
            log::error!(
                "Service with name '{}' already exists with index '{}'",
                &name,
                service_index.unwrap()
            );
            return;
        }

        let service = self.get_service_from_db(&name).await;
        let new_service: Arc<Mutex<Service>>;
        if let Some(service_schema) = service {
            log::info!(
                "Service with name '{}' already exists in database, recovering it..",
                &name
            );

            new_service = Arc::new(Mutex::new(Service::new(
                service_schema.id,
                service_schema.name,
                service_schema.enable,
                service_schema.creation_time,
                future_factory,
            )));
        } else {
            log::info!(
                "Service with name '{}' doesn't exist in database, creating it..",
                &name
            );

            let db = db::get_db().await;
            let insert_result = sqlx::query("INSERT INTO services (name, enable) VALUES (?, ?)")
                .bind(&name)
                .bind(enable)
                .execute(&db)
                .await;

            match insert_result {
                Ok(query_result) => {
                    log::info!(
                        "Insert of service with name: {} was successful! {:?}",
                        &name,
                        query_result
                    );
                }
                Err(err) => {
                    log::error!("Insert of service with name: {} failed! {:?}", &name, err);
                    return;
                }
            }
            let service_schema = self.get_service_from_db(&name).await.unwrap();
            new_service = Arc::new(Mutex::new(Service::new(
                service_schema.id,
                service_schema.name,
                service_schema.enable,
                service_schema.creation_time,
                future_factory,
            )));
        }
        {
            let mut service_lock = new_service.lock().await;
            service_lock.begin();
        }
        self.services.push(new_service);
    }

    pub async fn get_service(&self, name: &str) -> Option<Arc<Mutex<Service>>> {
        for service_guard in self.services.iter() {
            let service = service_guard.lock().await;
            if service.name == name {
                return Some(service_guard.clone());
            }
        }
        None
    }

    async fn get_service_internally(&self, name: &str) -> Option<usize> {
        for (i, service_guard) in self.services.iter().enumerate() {
            let service = service_guard.lock().await;
            if service.name == name {
                return Some(i);
            }
        }
        None
    }

    async fn get_service_from_db(&self, name: &str) -> Option<ServiceSchema> {
        let db: sqlx::Pool<Sqlite> = db::get_db().await;

        let service_query =
            sqlx::query_as::<_, ServiceSchema>("SELECT * FROM services WHERE name = ?")
                .bind(name)
                .fetch_one(&db)
                .await;

        match service_query {
            Ok(service) => {
                log::info!(
                    "[{}] name: {}, active {}",
                    service.id,
                    &service.name,
                    service.enable
                );
                Some(service)
            }
            Err(err) => {
                println!("{:?}", err);
                None
            }
        }
    }
}
