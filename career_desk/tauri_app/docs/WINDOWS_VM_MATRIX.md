# Windows 10/11 validation matrix

Run on clean, supported x64 virtual machines with current Windows updates and WebView2 Runtime. The hosted GitHub runners validate Windows Server environments; they are not substitutes for this client OS matrix.

| Profile | OS evidence | Automated command | Required manual checks |
|---|---|---|---|
| `win10` | Windows 10 edition, version, build, WebView2 version | `powershell -File scripts/windows_vm_matrix.ps1 -Profile win10` | NSIS install/current-user permissions, first launch, persistence after restart, external link confirmation, uninstall/reinstall |
| `win11` | Windows 11 edition, version, build, WebView2 version | `powershell -File scripts/windows_vm_matrix.ps1 -Profile win11` | Same checks plus SmartScreen/signature presentation and high-DPI layout |

Use `-RunDesktopProbe` only to collect the known upstream embedded WebDriver failure. A failed probe must remain failed in `result.json`; never edit the result to green. Until the driver issue is resolved, complete the functional desktop flows manually and attach screenshots/logs.

Minimum evidence per VM:

- generated `result.json` and command logs;
- OS/build and WebView2 versions;
- installer SHA-256 matched to release evidence;
- install/uninstall outcome and application launch screenshot;
- persistence/restart result;
- reviewer, timestamp, and any deviations.
