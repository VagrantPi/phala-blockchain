use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use docker_api::{
    api::{container::opts::ContainerCreateOpts, PublishPort},
    Docker,
};
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct Pod {
    pub image: String,
    pub container_id: String,
    pub id: String,
    pub tcp_portmap: Vec<(u16, u16)>,
}

pub struct TrackerInfo {
    pub tcp_ports_available: usize,
    pub pods_running: usize,
    pub pods_allocated: usize,
}

pub struct Tracker {
    docker: Docker,
    pods: HashMap<String, Pod>,
    available_tcp_ports: Vec<u16>,
    alloc_counter: usize,
}

impl Tracker {
    pub fn new(docker: Docker, (port_range_from, port_range_to): (u16, u16)) -> Tracker {
        Tracker {
            docker: docker,
            pods: Default::default(),
            available_tcp_ports: (port_range_from..=port_range_to).collect(),
            alloc_counter: 0,
        }
    }

    pub async fn create_pod<'a>(&'a mut self, image: &str, id: &str) -> Result<&'a Pod> {
        if self.pods.contains_key(id) {
            return Err(anyhow::anyhow!("Pod already exists"));
        }
        // TODO.kevin.must: get the port from somthing like manifest.json
        let required_ports = vec![80_u16];
        let exposed_ports = self
            .allocate_tcp_ports(required_ports.len())
            .ok_or(anyhow::anyhow!("No available ports"))?;
        let mut builder = ContainerCreateOpts::builder(image)
            .auto_remove(true)
            .tty(true);
        for (po, pi) in exposed_ports.iter().zip(required_ports.iter()) {
            builder = builder.expose(PublishPort::tcp(*po as _), *pi as _);
        }
        let opts = builder.build();
        let container = self.docker.containers().create(&opts).await?;
        container.start().await?;
        let pod = Pod {
            image: image.to_owned(),
            id: id.to_owned(),
            container_id: container.id().to_owned(),
            tcp_portmap: exposed_ports
                .into_iter()
                .zip(required_ports.into_iter())
                .collect(),
        };
        self.pods.insert(id.to_owned(), pod);
        Ok(self
            .pods
            .get(id)
            .expect("It must exists. We just inserted it"))
    }

    pub async fn stop_pod(&mut self, id: &str) -> Result<()> {
        let pod = self
            .pods
            .get(id)
            .ok_or(anyhow::anyhow!("Pod {} not found", id))?;
        let contrainer = self.docker.containers().get(pod.container_id.as_str());
        let wait = Duration::from_secs(5);
        // TODO.kevin.must: ignore error
        contrainer.stop(Some(wait)).await?;
        if let Some(pod) = self.pods.remove(id) {
            self.free_tcp_ports(pod.tcp_portmap.iter().map(|(s, _)| *s));
        }
        Ok(())
    }

    pub fn info(&self) -> TrackerInfo {
        TrackerInfo {
            tcp_ports_available: self.available_tcp_ports.len(),
            pods_running: self.pods.len(),
            pods_allocated: self.alloc_counter,
        }
    }

    pub fn pod_info<'a>(&'a self, id: &str) -> Option<&'a Pod> {
        self.pods.get(id)
    }

    pub fn iter_pods(&self) -> impl Iterator<Item = &Pod> {
        self.pods.values()
    }
}

impl Tracker {
    fn allocate_tcp_ports(&mut self, n: usize) -> Option<Vec<u16>> {
        if n > self.available_tcp_ports.len() {
            return None;
        }
        Some(self.available_tcp_ports.drain(0..n).collect())
    }

    fn free_tcp_ports(&mut self, ports: impl IntoIterator<Item = u16>) {
        self.available_tcp_ports.extend(ports);
    }
}