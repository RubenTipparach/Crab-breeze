## ADDED Requirements

### Requirement: A hit on a building changes no mesh
After a building's first damage, a change to which of its elements stand SHALL be drawn by rewriting the building's element state buffer only; its meshes SHALL NOT be rebuilt.

#### Scenario: A second shell on a damaged tower
- **WHEN** a shell breaks columns of a tower that is already damaged
- **THEN** the broken columns, and the furniture and rooms on the slabs that fall with them, disappear in the main pass, the shadows and the prepass, and no mesh of the tower is rebuilt

### Requirement: Every building vertex carries its element
Every vertex of a building's frame, room, furniture and skin meshes SHALL carry the slot of the element it hangs on (0 for always shown).

#### Scenario: Furniture on a slab
- **WHEN** a desk stands on slab (k, i, j)
- **THEN** its vertices carry that slab's slot, and it disappears when the slab does
