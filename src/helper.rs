use bevy::{ecs::error::CommandOutput, prelude::*};

pub struct CommandPipe<C, F>(C, F);

impl<C, F, B> Command for CommandPipe<C, F>
where
    C: Command,
    F: FnOnce(In<C::Out>, &mut World) -> B + Send + 'static,
    B: CommandOutput,
{
    type Out = B;

    fn apply(self, world: &mut World) -> B {
        let CommandPipe(c, f) = self;
        let a_result = c.apply(world);
        (f)(In(a_result), world)
    }
}

pub trait CommandExt: Command + Sized {
    /// Pass the output of this command into a function, creating a new compound command.
    fn pipe<B, F>(self, f: F) -> CommandPipe<Self, F>
    where
        F: FnOnce(In<Self::Out>, &mut World) -> B + Send + 'static;
}

impl<T: Command> CommandExt for T {
    fn pipe<B, F>(self, f: F) -> CommandPipe<Self, F>
    where
        F: FnOnce(In<Self::Out>, &mut World) -> B + Send + 'static,
    {
        CommandPipe(self, f)
    }
}
