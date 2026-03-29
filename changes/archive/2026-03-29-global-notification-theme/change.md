# Global Notification Theme

## Intent

Global notifications had no visual weight — just coloured text on the plain bar background. Error conditions (load failure, browse blocked, quit warning) were indistinguishable in urgency from informational messages. The exit warning used yellow rather than a clearly urgent colour.

## Approach

- Error notifications: light red text (`Rgb(255, 180, 180)`) on dark red background (`Rgb(100, 20, 20)`), message centred, seconds-remaining countdown flush right as `[N]`
- Info notifications: dim text, no background fill
- Pending-quit warning adopts Error style (absorbs `exit-warning-colour`)
- `Warning` and `Success` styles exist but are unused in the global bar — no treatment needed

## Log

Implemented in `src/main.rs` global status bar render block. Error and pending-quit now share the red background style. Countdown computed from `n.expires` each frame. Pending-quit warning timeout added (5s auto-dismiss). Browser-blocked warning extended to 3s. `exit-warning-colour` change absorbed and removed.

## Conclusion

Global notifications now clearly signal urgency through colour and background fill. Error conditions are immediately distinguishable from status messages.
