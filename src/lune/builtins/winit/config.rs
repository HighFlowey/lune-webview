use crate::lune::util::TableBuilder;
use mlua::prelude::*;

// EventLoopHandle
pub enum EventLoopHandle {
    Break,
}

impl LuaUserData for EventLoopHandle {}

// EventLoopMessage
#[derive(Clone, PartialEq)]
pub enum EventLoopMessage {
    CloseRequested,
    MouseButtton(String),
    KeyCode(String),
    None,
}

impl EventLoopMessage {
    pub fn create_lua_table(lua: &Lua) -> LuaResult<LuaTable> {
        TableBuilder::new(lua)?
            .with_value("CloseRequested", Self::CloseRequested)?
            .with_value("MouseButton", Self::MouseButtton("".into()))?
            .with_value("KeyCode", Self::KeyCode("".into()))?
            .with_value("None", Self::None)?
            .build_readonly()
    }
}

impl LuaUserData for EventLoopMessage {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method(
            "get_mousebutton",
            |lua: &Lua, this: &Self, _: ()| -> LuaResult<LuaValue> {
                match this {
                    EventLoopMessage::MouseButtton(button) => Ok(button.clone().into_lua(lua)?),
                    _ => Ok(LuaValue::Nil),
                }
            },
        );

        methods.add_method(
            "get_keycode",
            |lua: &Lua, this: &Self, _: ()| -> LuaResult<LuaValue> {
                match this {
                    EventLoopMessage::KeyCode(keycode) => Ok(keycode.clone().into_lua(lua)?),
                    _ => Ok(LuaValue::Nil),
                }
            },
        );

        methods.add_meta_method(
            "__eq",
            |_lua, this: &Self, other: LuaUserDataRef<'lua, Self>| {
                Ok(matches!(
                    (this, other.clone()),
                    (Self::CloseRequested, Self::CloseRequested)
                        | (Self::MouseButtton(_), Self::MouseButtton(_))
                        | (Self::KeyCode(_), Self::KeyCode(_))
                        | (Self::None, Self::None)
                ))
            },
        );

        methods.add_meta_method(
            "__tostring",
            |_lua: &Lua, this: &Self, _: ()| -> LuaResult<String> {
                Ok(match this {
                    EventLoopMessage::CloseRequested => "CloseRequested".to_string(),
                    EventLoopMessage::MouseButtton(button) => format!("MouseButton({})", button),
                    EventLoopMessage::KeyCode(keycode) => format!("KeyCode({})", keycode),
                    EventLoopMessage::None => "None".to_string(),
                })
            },
        );
    }
}
