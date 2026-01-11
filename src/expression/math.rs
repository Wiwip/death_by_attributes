#[cfg(test)]
mod tests {
    use super::*;
    use crate::ReflectAccessAttribute;
    use crate::condition::EvalContext;
    use crate::prelude::*;
    use crate::{AttributesRef, attribute};
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::{Single, World};

    attribute!(Int32Attr, i32);
    attribute!(UInt32Attr, u32);
    attribute!(Float32Attr, f32);
    attribute!(Float64Attr, f64);

    #[test]
    fn test_additions() {
        let mut world = World::new();

        world.spawn((Int32Attr::new(100), UInt32Attr::new(100)));

        world
            .run_system_once(|actor: Single<AttributesRef>| {
                /*let source = Int32Attr::source_expr();
                let target = Int32Attr::source_expr();

                let add_expr = source + target;

                let context = EvalContext {
                    target_actor: &actor,
                    source_actor: &actor,
                    owner: &actor,
                };

                let result = add_expr.eval(&context).unwrap();
                assert_eq!(result, 200);

                let source = Int32Attr::source_expr();
                let target = Int32Attr::source_expr();*/

                /*let mul_expr = source * target;
                let result = mul_expr.eval(&context).unwrap();
                assert_eq!(result, 10000);*/
            })
            .unwrap();
    }
}
