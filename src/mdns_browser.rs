use std::net::IpAddr;

use mdns_sd::{Receiver, ResolvedService, ServiceDaemon, ServiceEvent};

use crate::{
    AppResult,
    apperror::AppError::XCustomError,
    storage::{KnownHost, Store, StoreValue::Host},
};

pub struct MdnsBrowser {
    daemon: ServiceDaemon,
}

impl MdnsBrowser {
    pub fn new() -> AppResult<Self> {
        let daemon = ServiceDaemon::new()?;
        Ok(Self { daemon })
    }

    pub fn browse(&mut self) -> AppResult<Receiver<ServiceEvent>> {
        let receiver = self.daemon.browse("_remote-desktop._tcp.local.")?;
        Ok(receiver)
    }

    // pub fn verify(&self, instance_fullname: String) -> AppResult<()> {
    //     self.daemon
    //         .verify(instance_fullname, VERIFY_TIMEOUT_DEFAULT)?;
    //     Ok(())
    // }

    pub fn monitor_events(
        &mut self,
        receiver: Receiver<ServiceEvent>,
        dashmap: &Store,
    ) -> AppResult<()> {
        let now: std::time::Instant = std::time::Instant::now();
        while let Ok(event) = receiver.recv() {
            match event {
                ServiceEvent::ServiceResolved(info) => {
                    self.add_host(info.as_ref().clone(), dashmap.clone())?;
                }
                ServiceEvent::ServiceRemoved(service_type, fullname) => {
                    println!(
                        "At {:?} Service removed: {} {}",
                        now.elapsed(),
                        service_type,
                        fullname
                    );
                }
                other_event => {
                    println!("At {:?} Other event: {:?}", now.elapsed(), other_event);
                }
            }
        }
        Ok(())
    }

    fn add_host(&self, info: ResolvedService, dashmap: Store) -> AppResult<()> {
        println!("{:?}", info.txt_properties.clone().into_property_map_str());
        let hostname = info
            .get_hostname()
            .trim_end_matches('.')
            .trim_end_matches(".local")
            .to_string();
        let addr = match info
            .addresses
            .iter()
            .find(|e| e.is_ipv4())
            .ok_or(XCustomError("Address not found"))?
            .to_ip_addr()
        {
            IpAddr::V4(ipv4_addr) => ipv4_addr,
            _ => return Err(XCustomError("Invalid ip address")),
        };
        let properties = info.txt_properties.into_property_map_str();
        let knownhost = KnownHost {
            hostname,
            addr,
            quic_port: properties
                .get("quic_port")
                .ok_or(XCustomError("Port not found"))?
                .parse()?,
            encoded_cert: properties
                .get("encoded_fp")
                .ok_or(XCustomError("Certificate not found"))?
                .to_string(),
        };
        println!("{:?}", knownhost);
        dashmap.insert("host".into(), Host(knownhost));
        self.daemon.stop_browse("_remote-desktop._tcp.local.")?;
        Ok(())
    }
}
