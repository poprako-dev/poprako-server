// LEGACY DISABLED: Do not use. This file is intentionally commented out.
// use async_trait::async_trait;
// 
// use crate::domain::model::event::EventEmit;
// 
// #[async_trait]
// pub trait EffectSink {
//     /// Pulls events from the source and feeds them to the async channel for
//     /// background processing. Silently drops events when the channel is full
//     /// or already closed.
//     async fn handle<E>(&self, src: &mut E)
//     where
//         E: EventEmit + Send;
// }
// 
// #[async_trait]
// pub trait Effect {
//     /// Executes the side effect, without returning a result(keep siltent even when it fails).
//     async fn develop_effect<S>(&mut self, handler: &S)
//     where
//         S: EffectSink + Send + Sync;
// }
// 
// #[async_trait]
// impl<E> Effect for E
// where
//     E: EventEmit + Send,
// {
//     async fn develop_effect<S>(&mut self, handler: &S)
//     where
//         S: EffectSink + Send + Sync,
//     {
//         handler.handle(self).await;
//     }
// }
