## ADDED Requirements

### Requirement: The screen the pilot points at in free look is outlined

While seated with the cabin sealed and free look held, the MFD screen under the view's centre within 3 m SHALL show
a lit outline round it, and no other screen SHALL; the outlined screen SHALL be the one a click presses.

#### Scenario: Looking at the dash's centre screen
- **WHEN** the pilot holds free look and looks at `SCREEN_center_screen`
- **THEN** that screen is outlined in amber and the others are not

#### Scenario: Letting go of free look
- **WHEN** the pilot lets go of free look
- **THEN** no screen is outlined

### Requirement: Any MFD opens a menu of pages and apps

Clicking an MFD in free look SHALL show its menu: the readout pages and MAP, MAIL, SHOP and MENU; picking a row SHALL
put that page on that screen alone, and MENU SHALL open the game's pause menu.

#### Scenario: Putting the shop on the left MFD
- **WHEN** the pilot clicks the left hung MFD, then points at SHOP and clicks
- **THEN** the left MFD shows the shop and the other screens keep their pages

### Requirement: BACK gives a screen back the page it showed before its menu

BACK, or Escape while pointing at the screen, SHALL close that screen's menu or app and show the readout page the
screen showed when its menu was opened, whatever app pages were opened in between; the other screens SHALL keep
their pages.

#### Scenario: Leaving the shop
- **WHEN** the left MFD shows MECH, the pilot clicks it, picks SHOP, opens the Harrow's sheet and clicks BACK
- **THEN** the left MFD shows MECH again

#### Scenario: Closing the menu unused
- **WHEN** the centre MFD shows TARGET and the pilot clicks it, then presses Escape while pointing at it
- **THEN** the centre MFD shows TARGET again
