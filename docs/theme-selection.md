# Appearance themes

Settings → Appearance provides Forest, Midnight and Paper. Selection applies immediately, persists on this device, and supports keyboard arrows. Unknown saved values fall back to Forest. With blocked storage the selection remains available during the current session.

The entire workspace, including Study, uses shared semantic color tokens. Positive/negative results keep green/red meaning; playing-card suits remain fixed. No new dependencies.

Implementation follows [CSS custom properties](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Cascading_variables/Using_custom_properties) and [color-scheme](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/color-scheme).

Paper (white) is the default for new or invalid preferences in v0.2.0. Valid saved choices remain unchanged; startup HTML and the native window use Paper colors.
