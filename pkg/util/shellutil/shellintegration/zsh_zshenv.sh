# Store the initial ZDOTDIR value
AGENTMUX_ZDOTDIR="$ZDOTDIR"

# Source the original zshenv
[ -f ~/.zshenv ] && source ~/.zshenv

# Detect if ZDOTDIR has changed
if [ "$ZDOTDIR" != "$AGENTMUX_ZDOTDIR" ]; then
  # If changed, manually source your custom zshrc from the original AGENTMUX_ZDOTDIR
  [ -f "$AGENTMUX_ZDOTDIR/.zshrc" ] && source "$AGENTMUX_ZDOTDIR/.zshrc"
fi