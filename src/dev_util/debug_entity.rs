use bevy::prelude::*;

pub trait WorldExt {
    fn debug_entity(&self, entity: Entity) -> Result<DebugEntity>;
}

#[derive(Debug)]
pub struct DebugEntity {
    pub id: Entity,
    pub name: Option<String>,
    pub components: Vec<String>,
}

impl std::fmt::Display for DebugEntity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = &self.name {
            f.write_str(name)?;
        } else {
            f.write_str("Entity")?;
        }
        write!(f, "({:?}) ", self.id)?;
        for (i, component) in self.components.iter().enumerate() {
            f.write_str(component)?;
            if i + 1 != self.components.len() {
                f.write_str(", ")?;
            }
        }
        Ok(())
    }
}

impl WorldExt for World {
    fn debug_entity(&self, entity: Entity) -> Result<DebugEntity> {
        let entity_ref = self.get_entity(entity)?;
        let component_ids = entity_ref.archetype().components();
        let component_names = component_ids
            .iter()
            .filter_map(|&id| {
                self.components()
                    .get_info(id)
                    .map(|info| info.name().to_string())
            })
            .collect::<Vec<_>>();
        Ok(DebugEntity {
            id: entity,
            name: self.get::<Name>(entity).map(|n| n.to_string()),
            components: component_names,
        })
    }
}
