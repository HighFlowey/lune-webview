mod config;
mod window;

use crate::lune::util::TableBuilder;
use mlua::prelude::*;
use mlua_luau_scheduler::LuaSpawnExt;
use once_cell::sync::Lazy;
use std::{cell::RefCell, rc::Weak, time::Duration};
use winit::{
    event_loop::{EventLoop, EventLoopBuilder},
    platform::pump_events::EventLoopExtPumpEvents,
    window::WindowId,
};

use self::{
    config::{EventLoopHandle, EventLoopMessage},
    window::config::LuaWindow,
};

pub static EVENT_LOOP_SENDER: Lazy<
    tokio::sync::watch::Sender<(Option<WindowId>, EventLoopMessage)>,
> = Lazy::new(|| {
    let init = (None, EventLoopMessage::None);
    tokio::sync::watch::Sender::new(init)
});

pub fn create(lua: &Lua) -> LuaResult<LuaTable> {
    let events = EventLoopMessage::create_lua_table(lua)?;

    TableBuilder::new(lua)?
        .with_value("events", events)?
        .with_async_function("event_loop", winit_event_loop)?
        .with_async_function("run", winit_run)?
        .with_function("new", winit_new)?
        .build_readonly()
}

thread_local! {
    pub static EVENT_LOOP: RefCell<EventLoop<()>> = RefCell::new(EventLoopBuilder::new().build().unwrap());
}

pub fn winit_new(lua: &Lua, _: ()) -> LuaResult<LuaAnyUserData> {
    window::create(lua)
}

pub async fn winit_run(lua: &Lua, _: ()) -> LuaResult<()> {
    lua.spawn_local(async {
        loop {
            let mut message: (Option<WindowId>, EventLoopMessage) = (None, EventLoopMessage::None);

            EVENT_LOOP.with(|event_loop| {
                let mut event_loop = event_loop.borrow_mut();

                event_loop.pump_events(Some(Duration::ZERO), |event, _elwt| {
                    if let winit::event::Event::WindowEvent {
                        window_id,
                        event: winit::event::WindowEvent::CloseRequested,
                    } = event
                    {
                        message = (Some(window_id), EventLoopMessage::CloseRequested);
                    }
                });
            });

            if EVENT_LOOP_SENDER.receiver_count() > 0 {
                EVENT_LOOP_SENDER.send(message).unwrap();
            } else {
                break;
            }

            tokio::time::sleep(Duration::ZERO).await;
        }
    });

    Ok(())
}

pub async fn winit_event_loop(lua: &Lua, values: LuaMultiValue<'_>) -> LuaResult<()> {
    let field1 = values.get(0).expect("Parameter 1 is missing");
    let field2 = values.get(1).expect("Parameter 2 is missing");

    let (window_key, callback_key) = {
        let window_key = lua.create_registry_value(field1)?;
        let callback_key = lua.create_registry_value(field2)?;
        (window_key, callback_key)
    };

    let inner_lua = lua
        .app_data_ref::<Weak<Lua>>()
        .expect("Missing weak lua ref")
        .upgrade()
        .expect("Lua was dropped unexpectedly");

    lua.spawn_local(async move {
        let mut listener = EVENT_LOOP_SENDER.subscribe();

        let inner_field1 = inner_lua.registry_value::<LuaValue>(&window_key).unwrap();
        let inner_field2 = inner_lua.registry_value::<LuaValue>(&callback_key).unwrap();

        loop {
            let changed = listener.changed().await;

            if changed.is_ok() {
                let (window, callback) = {
                    let window = inner_field1
                        .as_userdata()
                        .unwrap()
                        .borrow::<LuaWindow>()
                        .unwrap();

                    let callback = inner_field2.as_function().unwrap();

                    (window, callback.clone())
                };

                let message = *listener.borrow_and_update();

                if let Some(window_id) = message.0 {
                    if window.window.id() != window_id {
                        drop(window);
                        continue;
                    }
                }

                window.window.request_redraw();
                drop(window);

                let callback_future =
                    callback.call_async::<_, LuaValue>((EventLoopHandle::Break, message.1));

                let callback_result = callback_future.await.unwrap();

                if let Some(userdata) = callback_result.as_userdata() {
                    if let Ok(handle) = userdata.borrow::<EventLoopHandle>() {
                        match *handle {
                            EventLoopHandle::Break => break,
                        }
                    }
                }
            }

            tokio::time::sleep(Duration::ZERO).await;
        }
    });

    Ok(())
}