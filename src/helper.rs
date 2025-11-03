use std::marker::PhantomData;

use bevy::prelude::*;

pub struct CommandPipe<C, F, A>(C, F, PhantomData<A>);

impl<C, F, A, B> Command<B> for CommandPipe<C, F, A>
where
    A: Send + 'static,
    C: Command<A>,
    F: FnOnce(In<A>, &mut World) -> B + Send + 'static,
{
    fn apply(self, world: &mut World) -> B {
        let CommandPipe(c, f, _) = self;
        let a_result = c.apply(world);
        (f)(In(a_result), world)
    }
}

pub trait CommandExt: Sized {
    /// Pass the output of this command into a function, creating a new compound command.
    fn pipe<A, B, F>(self, f: F) -> CommandPipe<Self, F, A>
    where
        Self: Command<A>,
        F: FnOnce(In<A>, &mut World) -> B + Send + 'static;
}

impl<T: Sized> CommandExt for T {
    fn pipe<A, B, F>(self, f: F) -> CommandPipe<Self, F, A>
    where
        Self: Command<A>,
        F: FnOnce(In<A>, &mut World) -> B + Send + 'static,
    {
        CommandPipe(self, f, PhantomData)
    }
}
