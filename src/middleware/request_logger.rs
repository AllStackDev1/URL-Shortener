use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::Error;
use futures_util::future::{ok, LocalBoxFuture, Ready};
use std::rc::Rc;

use log::debug;

pub struct RequestLogger {
    enable_debug_logging: bool,
}

impl RequestLogger {
    pub fn new(enable_debug_logging: bool) -> Self {
        Self { enable_debug_logging }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RequestLogger
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RequestLoggerMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RequestLoggerMiddleware {
            service: Rc::new(service),
            enable_debug_logging: self.enable_debug_logging,
        })
    }
}

pub struct RequestLoggerMiddleware<S> {
    service: Rc<S>,
    enable_debug_logging: bool,
}

impl<S, B> Service<ServiceRequest> for RequestLoggerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let enable_debug_logging = self.enable_debug_logging;

        if enable_debug_logging {
            let path = req.path().to_owned();
            let method = req.method().clone();

            debug!("Processing request: {} {}", method, path);

            Box::pin(async move {
                let res = service.call(req).await?;
                debug!("Response: {} {} - status: {}", method, path, res.status());
                Ok(res)
            })
        } else {
            Box::pin(service.call(req))
        }
    }
}