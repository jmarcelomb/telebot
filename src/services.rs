use crate::db;
use core::future::Future;
use futures::future::{AbortHandle, Abortable};
use futures::stream::Aborted;
use sqlx::{FromRow, Sqlite};
use std::sync::Arc;
use tokio;
use tokio::sync::Mutex;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

#[derive(Clone, FromRow, Debug)]
struct ServiceSchema {
    id: i64,
    name: String,
    enable: bool,
    creation_time: String,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum ServiceState {
    Paused,
    Running,
    Sleeping,
}

#[derive(Clone, Debug)]
pub struct Service {
    pub id: i64,
    pub name: String,
    pub enable: bool,
    pub creation_time: String,
    pub manager: WorkerManagement,
    state: ServiceState,
    next_state: ServiceState,
}

async fn wake_up_thread_in(duration: Duration, notify: Arc<Notify>) {
    log::info!("In wake up thread in, before sleep");
    sleep(duration).await;
    log::info!("In wake up thread in, after sleep");
    notify.notify_waiters();
    log::info!("In wake up thread in, After notifying waiters");
}

pub async fn manager(service_name: &str) {
    let service_guard_option;
    {
        let services = crate::get_services().write().await;
        service_guard_option = services.get_service(&service_name).await;
    }
    let mut sleep_future_handle = None;
    let mut notify = None;
    if let Some(service_guard) = service_guard_option {
        let mut service = service_guard.lock().await;
        notify = Some(service.manager.notify.clone());
        let next_state = service.next_state();
        service.set_state(next_state);
        match service.state {
            ServiceState::Paused => service.set_next_state(ServiceState::Running),
            ServiceState::Running => {
                return;
            }
            ServiceState::Sleeping => {
                service.set_next_state(ServiceState::Running);
                sleep_future_handle = service.get_sleep_future().await;
            }
        }
    }
    if notify.is_some() {
        notify.unwrap().notified().await;
    }

    if let Some(join_handle) = sleep_future_handle {
        let result = join_handle.await;

        match result {
            Ok(_) => log::info!("Wake-up thread completed successfully"),
            Err(_) => log::warn!("Wake-up thread was aborted"),
        }
    }
    Box::pin(manager(service_name)).await;
}

pub async fn manager_2(service_name: &str) {
    let mut notify = None;
    let mut join_handle = None;
    {
        let service;
        {
            let services = crate::get_services().write().await;
            log::info!("Acquired write lock on services");

            service = services.get_service(&service_name).await;
            log::info!("Retrieved service '{}' from services", service_name);
        }
        if service.is_some() {
            log::info!("Service '{}' exists", service_name);
            let service_guard = service.unwrap();
            let mut service = service_guard.lock().await;
            notify = Some(service.manager.notify.clone());
            log::info!("Acquiring lock on service '{}'", service_name);
            if !service.manager.paused {
                join_handle = service.get_sleep_future().await;
            }
            log::info!("Finished supervisor() on service '{}'", service_name);
        } else {
            log::info!("Service '{}' does not exist", service_name);
        }
    }
    if join_handle.is_some() {
        let result = join_handle.unwrap().await;
        match result {
            Ok(_) => log::info!("Wake-up thread completed successfully"),
            Err(_) => log::warn!("Wake-up thread was aborted"),
        }
    } else if notify.is_some() {
        notify.unwrap().notified().await;
    }
}

impl Service {
    pub fn new(
        id: i64,
        name: String,
        enable: bool,
        creation_time: String,
        manager: WorkerManagement,
    ) -> Self {
        let service_state = if enable == true {
            ServiceState::Running
        } else {
            ServiceState::Paused
        };
        Self {
            id,
            name,
            enable,
            creation_time,
            manager,
            state: service_state,
            next_state: service_state,
        }
    }
    pub async fn get_sleep_future(&mut self) -> Option<JoinHandle<Result<(), Aborted>>> {
        if self.manager.sleep_duration.is_some() {
            let (abort_handle, abort_registration) = AbortHandle::new_pair();
            self.manager.abort_handle = Some(abort_handle);
            let sleep_duration = self.manager.sleep_duration.unwrap();
            let notify_clone = self.manager.notify.clone();

            let abortable_sleep = Abortable::new(
                async move {
                    wake_up_thread_in(sleep_duration, notify_clone).await;
                },
                abort_registration,
            );

            let handle = tokio::spawn(abortable_sleep);
            return Some(handle);
        }
        return None;
    }
    fn set_state(&mut self, state: ServiceState) {
        self.state = state;
    }
    fn set_next_state(&mut self, next_state: ServiceState) {
        self.next_state = next_state;
    }
    pub fn update_state(&mut self) {
        self.state = self.next_state;
    }
    fn next_state(&self) -> ServiceState {
        match self.state {
            ServiceState::Running => {
                if self.manager.paused == true {
                    return ServiceState::Paused;
                } else if self.manager.sleep_duration.is_some() {
                    return ServiceState::Sleeping;
                } else {
                    return ServiceState::Running;
                }
            }
            ServiceState::Paused => {
                if self.manager.paused == true {
                    return ServiceState::Paused;
                }
                return ServiceState::Running;
            }
            ServiceState::Sleeping => {
                if self.manager.paused == true {
                    return ServiceState::Paused;
                }
                return ServiceState::Running;
            }
        }
    }

    pub async fn set_enable_state(&mut self, state: bool) {
        self.enable = state;
        let previous_paused = self.manager.paused;
        self.manager.paused = !self.enable;
        if previous_paused && state == true {
            self.manager.notify.notify_waiters();
        } else if self.state == ServiceState::Sleeping {
            if let Some(abort_handle) = &self.manager.abort_handle {
                abort_handle.abort();
            }
        }

        let db = db::get_db().await;
        let update_enable_state = sqlx::query("UPDATE services SET enable = False WHERE id = ?;")
            .bind(&self.id)
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
                return;
            }
        }
    }
}

pub struct Services {
    pub services: Vec<Arc<Mutex<Service>>>,
}

impl Services {
    pub fn new() -> Self {
        Self {
            services: Vec::new(),
        }
    }

    pub async fn create_service<F>(
        &mut self,
        name: String,
        mut worker_manager: WorkerManagement,
        future: F,
    ) where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
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
        if service.is_some() {
            log::info!(
                "Service with name '{}' already exists in database, recovering it..",
                &name
            );
            let service_schema = service.unwrap();
            worker_manager.paused = !service_schema.enable;

            new_service = Arc::new(Mutex::new(Service::new(
                service_schema.id,
                service_schema.name,
                service_schema.enable,
                service_schema.creation_time,
                worker_manager,
            )));
        } else {
            log::info!(
                "Service with name '{}' doesn't exists in database, creating it..",
                &name
            );

            let db = db::get_db().await;
            let insert_result = sqlx::query("INSERT INTO services (name, enable) VALUES (?, ?)")
                .bind(&name)
                .bind(worker_manager.paused)
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
                worker_manager,
            )));
        }
        self.services.push(new_service);
        tokio::spawn(future);
    }

    pub async fn get_service(&self, name: &str) -> Option<Arc<Mutex<Service>>> {
        for service_guard in self.services.iter() {
            let service = service_guard.lock().await;
            if service.name == name {
                return Some(service_guard.clone());
            }
        }
        return None;
    }

    async fn get_service_internally(&self, name: &str) -> Option<usize> {
        for (i, service_guard) in self.services.iter().enumerate() {
            let service = service_guard.lock().await;
            if service.name == name {
                return Some(i);
            }
        }
        return None;
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
                return Some(service);
            }
            Err(err) => {
                println!("{:?}", err);
            }
        }
        return None;
    }
}

#[derive(Clone, Debug)]
pub struct WorkerManagement {
    pub paused: bool,
    pub abort_handle: Option<AbortHandle>,
    pub notify: Arc<Notify>,
    pub sleep_duration: Option<Duration>,
}

impl WorkerManagement {
    pub fn new(paused: bool, duration: Duration) -> Self {
        Self {
            paused: paused,
            abort_handle: None,
            notify: Arc::new(Notify::new()),
            sleep_duration: Some(duration),
        }
    }
}
impl Default for WorkerManagement {
    fn default() -> Self {
        Self {
            paused: false,
            abort_handle: None,
            notify: Arc::new(Notify::new()),
            sleep_duration: None,
        }
    }
}
