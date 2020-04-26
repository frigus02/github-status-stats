function CreatePNG([int]$DPI, [int]$Size) {
	& inkscape `
		--without-gui `
		--export-png="$PSScriptRoot/../res/favicon-$Size.png" `
		--export-area-page `
		--export-dpi=$DPI `
		"$PSScriptRoot/../res/icon.svg"
}

CreatePNG 96 16
CreatePNG 192 32

imconvert `
	"$PSScriptRoot/../res/favicon-16.png" `
	"$PSScriptRoot/../res/favicon-32.png" `
	"$PSScriptRoot/../crates/ghss_website/static/favicon.ico"

rm "$PSScriptRoot/../res/favicon-16.png"
rm "$PSScriptRoot/../res/favicon-32.png"
