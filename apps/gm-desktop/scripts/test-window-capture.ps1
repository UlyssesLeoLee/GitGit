#requires -Version 7
<#
.SYNOPSIS
    Exercise the pixel half of the packaged-app launch proof with synthetic
    captures.

.DESCRIPTION
    WHY THIS TEST EXISTS
    ====================
    The `msi-launch` job captures the packaged application's window and
    asserts that it shows the product. A gate that has never been fed a
    failure is not known to be a gate, so the decision has to be shown to
    REJECT the things this lane exists to catch.

    The decision lives in `window-capture.ps1`, which has no side effects and
    needs no Windows desktop, so these cases run anywhere PowerShell runs —
    including the Linux runner of the always-on `gate` workflow, where every
    pull request is measured.

    [FACT] These cases are about the DECISION, not about the capture: the
    bytes of a capture are handed to the decision directly. The capture
    itself is exercised on Windows, in the `msi-launch` job, which captures a
    real blank window through the real capture path and asserts that the same
    decision rejects it.

    THE ERROR CASE MODELS THE REAL COMPOSITION
    ===========================================
    [FACT] The application is a SHELL - a sidebar of `w-56` and a header of
    `h-12` - around the area the route renders into, and the shell paints
    accent-coloured controls of its own: the selected theme and locale
    toggles in the sidebar, and the header's primary button and server
    indicator.

    [FACT] Measured 2026-11-05 on the development machine, on a capture of
    the running application (1296x839 window, 1280x800 client area):
        the whole window          6753 accent pixels,  93 colour regions
        the sidebar (x < 224)     2225 accent pixels
        the header  (y <  48)     2264 accent pixels
        the route content area    2264 accent pixels,  15 colour regions
    The shell alone supplies 4489 accent pixels - 29x the threshold of 150 -
    so a WHOLE-WINDOW measurement of the accent colour is satisfied by a
    screen whose route content shows an error boundary. That is the defect
    this case now reproduces: the error composition below is built with the
    shell's accent-coloured controls in place, and the assertion is that the
    route content area of that composition holds NO accent colour, so the
    decision rejects it.

    An earlier version of this case was a white surface with red text on it.
    That was not what the application looks like when a route throws, so the
    case never exercised the failure it claimed to.

    THE CASES
    =========
    Each of them is a failure the check has to catch:
      1. a flat white surface — the webview that loaded nothing;
      2. a flat surface in the product's accent colour — a splash screen;
      3. a white page with the native title bar around it — the exact thing
         a "is it one flat colour?" check is fooled by;
      4. the REAL fallback composition — the shell with its accent-coloured
         controls, and an error card where the route content should be — which
         has to be REJECTED;
      5. the product, rendered — which has to be ACCEPTED, so the check is
         not simply refusing everything;
      6. the product without its accent-coloured control — which has to be
         rejected, so the check is not simply counting green pixels.

.NOTES
    Exit codes
        0  every case behaved as specified.
        1  at least one case did not; each failure is printed.
        2  setup error (the helper is not there, or the palette could not be
           read).
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$helper = Join-Path $PSScriptRoot 'window-capture.ps1'
if (-not (Test-Path -LiteralPath $helper)) {
    Write-Error "the window-capture helper '$helper' is not there"
    exit 2
}
. $helper

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..' '..' '..')).Path
$tailwindConfig = Join-Path $repoRoot 'apps/gm-desktop/tailwind.config.js'
$sidebarSource = Join-Path $repoRoot 'apps/gm-desktop/src/lib/components/Sidebar.svelte'
$statusBarSource = Join-Path $repoRoot 'apps/gm-desktop/src/lib/components/ServerStatusBar.svelte'

$script:Failures = 0
$script:CaseCount = 0

function Write-Case { param([string]$M) Write-Host "    --  $M" -ForegroundColor DarkGray }
function Write-Ok   { param([string]$M) Write-Host "    ok  $M" -ForegroundColor Green }
function Write-Bad  { param([string]$M) Write-Host "    FAIL  $M" -ForegroundColor Red; $script:Failures++ }

function Assert-True {
    param([bool]$Condition, [string]$Because)
    $script:CaseCount++
    if ($Condition) { Write-Ok $Because } else { Write-Bad $Because }
}

function Assert-Equal {
    param($Expected, $Actual, [string]$Because)
    $script:CaseCount++
    if ($Expected -eq $Actual) {
        Write-Ok $Because
    } else {
        Write-Bad ("{0} (expected '{1}', got '{2}')" -f $Because, $Expected, $Actual)
    }
}

# ---------------------------------------------------------------------------
# Synthetic captures.
# ---------------------------------------------------------------------------
# [FACT] The buffers are built here rather than in `window-capture.ps1`,
# because they are a property of the TEST and not of the product.
$builder = @'
using System;

// [FACT] The buffers are in BGRA order, because that is the order GDI hands
// pixels over in and the order `Format32bppArgb` keeps in memory — the same
// order a real capture arrives in.
public static class GitGitSyntheticCapture
{
    /// <summary>A surface of one colour.</summary>
    public static byte[] Flat(int width, int height, int r, int g, int b)
    {
        byte[] px = new byte[width * height * 4];
        for (int i = 0; i < px.Length; i += 4)
        {
            px[i] = (byte)b; px[i + 1] = (byte)g; px[i + 2] = (byte)r; px[i + 3] = 255;
        }
        return px;
    }

    /// <summary>A rectangle of one colour on a surface of another.</summary>
    public static byte[] Rect(int width, int height, int br, int bg, int bb,
                              int x0, int y0, int rw, int rh, int r, int g, int b)
    {
        byte[] px = Flat(width, height, br, bg, bb);
        Fill(px, width, height, x0, y0, x0 + rw, y0 + rh, r, g, b);
        return px;
    }

    /// <summary>
    /// The REAL fallback composition: the application shell, with the
    /// accent-coloured controls the shell paints of its own, and an error
    /// card where the route content should be.
    /// </summary>
    /// <remarks>
    /// [FACT] The colours are the product's own, measured 2026-11-05 on a
    /// capture of the running application: the shell's selected theme and
    /// locale toggles are the brand green `#22c55e`, the sidebar's active
    /// navigation link is `#15803d` on the palest step `#f0fdf4`, and the
    /// error card is `#dc2626` text on a `#fef2f2` field with a `#fca5a5`
    /// border — none of which is the brand green.
    /// </remarks>
    public static byte[] ErrorFallback(int width, int height,
                                       int sidebarWidth, int headerHeight,
                                       int accentR, int accentG, int accentB)
    {
        byte[] px = Flat(width, height, 255, 255, 255);

        // The main panel's background, and the header's, and the sidebar's.
        Fill(px, width, height, sidebarWidth, 0, width, height, 248, 250, 252);
        Fill(px, width, height, sidebarWidth, 0, width, headerHeight, 255, 255, 255);
        Fill(px, width, height, 0, 0, sidebarWidth, height, 255, 255, 255);
        // The shell's edges.
        Fill(px, width, height, sidebarWidth - 1, 0, sidebarWidth, height, 226, 232, 240);
        Fill(px, width, height, sidebarWidth, headerHeight - 1, width, headerHeight, 226, 232, 240);

        // The sidebar's selected theme toggle and locale switcher: the
        // SHELL paints these, in the brand green, whatever the route shows.
        Fill(px, width, height, 16, height - 120, 100, height - 96, accentR, accentG, accentB);
        Fill(px, width, height, 16, height - 80, 84, height - 56, accentR, accentG, accentB);
        // The sidebar's active navigation link: the palest step with the
        // darker green text.
        Fill(px, width, height, 16, 48, 96, 88, 240, 253, 244);
        Fill(px, width, height, 24, 56, 72, 80, 21, 128, 61);
        // The header's server indicator and its primary button.
        Fill(px, width, height, sidebarWidth + 16, 18, sidebarWidth + 26, 28, accentR, accentG, accentB);
        Fill(px, width, height, width - 96, 8, width - 16, 40, accentR, accentG, accentB);

        // The ERROR CARD, where the route content would be: red, not green.
        int cardX = sidebarWidth + 24;
        int cardY = headerHeight + 24;
        Fill(px, width, height, cardX, cardY, cardX + 360, cardY + 120, 254, 242, 242);
        Fill(px, width, height, cardX, cardY, cardX + 360, cardY + 1, 252, 165, 165);
        Fill(px, width, height, cardX, cardY + 119, cardX + 360, cardY + 120, 252, 165, 165);
        Fill(px, width, height, cardX, cardY + 1, cardX + 1, cardY + 119, 252, 165, 165);
        Fill(px, width, height, cardX + 359, cardY + 1, cardX + 360, cardY + 119, 252, 165, 165);
        // The heading and the message, in red.
        Fill(px, width, height, cardX + 16, cardY + 16, cardX + 180, cardY + 32, 185, 28, 28);
        Fill(px, width, height, cardX + 16, cardY + 44, cardX + 300, cardY + 56, 220, 38, 38);
        // The retry button, in slate.
        Fill(px, width, height, cardX + 16, cardY + 80, cardX + 100, cardY + 104, 226, 232, 240);

        return px;
    }

    /// <summary>
    /// The product, rendered: the same shell, with route content that
    /// carries the brand green.
    /// </summary>
    public static byte[] Product(int width, int height,
                                 int sidebarWidth, int headerHeight,
                                 int accentR, int accentG, int accentB,
                                 bool withAccent)
    {
        byte[] px = Flat(width, height, 255, 255, 255);

        Fill(px, width, height, sidebarWidth, 0, width, height, 248, 250, 252);
        Fill(px, width, height, sidebarWidth, 0, width, headerHeight, 255, 255, 255);
        Fill(px, width, height, 0, 0, sidebarWidth, height, 255, 255, 255);
        Fill(px, width, height, sidebarWidth - 1, 0, sidebarWidth, height, 226, 232, 240);
        Fill(px, width, height, sidebarWidth, headerHeight - 1, width, headerHeight, 226, 232, 240);

        // The shell's own accent-coloured controls.
        Fill(px, width, height, 16, height - 120, 100, height - 96, accentR, accentG, accentB);
        Fill(px, width, height, 16, height - 80, 84, height - 56, accentR, accentG, accentB);
        Fill(px, width, height, 16, 48, 96, 88, 240, 253, 244);
        Fill(px, width, height, 24, 56, 72, 80, 21, 128, 61);
        Fill(px, width, height, sidebarWidth + 16, 18, sidebarWidth + 26, 28, accentR, accentG, accentB);
        Fill(px, width, height, width - 96, 8, width - 16, 40, accentR, accentG, accentB);

        // The route content: a card with text-like bands, and - when the
        // product is rendered - the brand green.
        int cardX = sidebarWidth + 24;
        int cardY = headerHeight + 24;
        Fill(px, width, height, cardX, cardY, cardX + 360, cardY + 200, 255, 255, 255);
        Fill(px, width, height, cardX, cardY, cardX + 360, cardY + 1, 226, 232, 240);
        Fill(px, width, height, cardX, cardY + 199, cardX + 360, cardY + 200, 226, 232, 240);
        for (int i = 0; i < 12; i++)
        {
            int y = cardY + 20 + i * 12;
            int shade = 60 + (i * 13) % 140;
            Fill(px, width, height, cardX + 16, y, cardX + 16 + 60 + (i * 9) % 200, y + 4,
                 shade, (int)(shade * 1.1), (int)(shade * 1.4));
        }
        if (withAccent)
        {
            Fill(px, width, height, cardX + 16, cardY + 168, cardX + 88, cardY + 192,
                 accentR, accentG, accentB);
        }

        return px;
    }

    /// <summary>The native chrome around a white client area.</summary>
    public static byte[] ChromeAndWhite(int width, int height, int titleBarHeight)
    {
        byte[] px = Flat(width, height, 255, 255, 255);
        // #F3F3FF — the colour the Windows 11 title bar paints, measured.
        Fill(px, width, height, 0, 0, width, titleBarHeight, 243, 243, 255);
        return px;
    }

    static void Fill(byte[] px, int width, int height, int x0, int y0, int x1, int y1,
                     int r, int g, int b)
    {
        for (int y = Math.Max(0, y0); y < Math.Min(height, y1); y++)
        {
            for (int x = Math.Max(0, x0); x < Math.Min(width, x1); x++)
            {
                int i = (y * width + x) * 4;
                px[i] = (byte)b; px[i + 1] = (byte)g; px[i + 2] = (byte)r; px[i + 3] = 255;
            }
        }
    }
}
'@

if (-not ('GitGitSyntheticCapture' -as [type])) {
    Add-Type -TypeDefinition $builder -ErrorAction Stop
}

# ---------------------------------------------------------------------------
# The palette and the layout the check is measured against.
# ---------------------------------------------------------------------------
Write-Case "the accent palette is read out of the product's own configuration"
try {
    $accent = @(Get-ProductAccentColors -TailwindConfigPath $tailwindConfig)
} catch {
    Write-Error "the accent palette could not be read: $_"
    exit 2
}
Assert-True ($accent.Count -ge 1) 'the accent palette carries at least one chromatic colour'

$hexes = @($accent | ForEach-Object { $_.Hex.ToLowerInvariant() })
Assert-True ($hexes -contains '#22c55e') "the brand green #22c55e is one of the colours checked for"
Assert-True ($hexes -notcontains '#f0fdf4') 'the palest accent step is NOT checked for, because the native title bar matches it'
Assert-True ($hexes -notcontains '#dcfce7') 'the second-palest accent step is NOT checked for either'

Write-Case "the shell's size is read out of the product's own components"
try {
    $layout = Get-ProductShellLayout -SidebarSourcePath $sidebarSource -StatusBarSourcePath $statusBarSource
} catch {
    Write-Error "the product's shell layout could not be read: $_"
    exit 2
}
Assert-Equal 224 $layout.SidebarWidthPx 'the sidebar is 224 CSS pixels wide (w-56)'
Assert-Equal 48 $layout.HeaderHeightPx 'the header is 48 CSS pixels tall (h-12)'

$width = 1280
$height = 800
$accentR = 0x22; $accentG = 0xc5; $accentB = 0x5e
$sidebarWidth = $layout.SidebarWidthPx
$headerHeight = $layout.HeaderHeightPx

function Measure-Buffer {
    param([byte[]]$Pixels, [int]$W = 1280, [int]$H = 800)
    # [FACT] The accent colour is counted in the ROUTE CONTENT AREA: the
    # client area less the shell. The synthetic buffers ARE the client area,
    # so the region's origin is the shell's own size.
    return Measure-CapturedPixels -Pixels $Pixels -Width $W -Height $H -AccentColors $accent `
        -RegionX0 $sidebarWidth -RegionY0 $headerHeight -RegionX1 $W -RegionY1 $H
}

function Measure-WholeWindow {
    param([byte[]]$Pixels, [int]$W = 1280, [int]$H = 800)
    return Measure-CapturedPixels -Pixels $Pixels -Width $W -Height $H -AccentColors $accent
}

function Verdict {
    param($Measurement)
    return Test-CapturedProduct -Measurement $Measurement
}

# ---------------------------------------------------------------------------
# 1. a flat white surface
# ---------------------------------------------------------------------------
Write-Case 'a flat white surface is not the product'
$flatWhite = [GitGitSyntheticCapture]::Flat($width, $height, 255, 255, 255)
$measurement = Measure-Buffer -Pixels $flatWhite
$verdict = Verdict -Measurement $measurement
Assert-Equal 0 $measurement.AccentPixels 'the flat white surface holds no accent pixels'
Assert-Equal 1 $measurement.ColourRegions 'the flat white surface is one colour region'
Assert-True (-not $verdict.Passed) 'the flat white surface is rejected'

# ---------------------------------------------------------------------------
# 2. a flat surface in the accent colour (a splash screen)
# ---------------------------------------------------------------------------
Write-Case 'a flat surface in the accent colour is not the product either'
$flatAccent = [GitGitSyntheticCapture]::Flat($width, $height, $accentR, $accentG, $accentB)
$measurement = Measure-Buffer -Pixels $flatAccent
$verdict = Verdict -Measurement $measurement
Assert-True ($measurement.AccentPixels -gt 0) 'the flat accent surface holds accent pixels'
Assert-Equal 1 $measurement.ColourRegions 'the flat accent surface is one colour region'
Assert-True (-not $verdict.Passed) 'the flat accent surface is rejected'

# ---------------------------------------------------------------------------
# 3. a white page with the native title bar around it
# ---------------------------------------------------------------------------
Write-Case 'a white page with the native title bar is not the product'
$chrome = [GitGitSyntheticCapture]::ChromeAndWhite($width, $height, 30)
$measurement = Measure-Buffer -Pixels $chrome
$verdict = Verdict -Measurement $measurement
Assert-Equal 0 $measurement.AccentPixels 'the native title bar colour is not mistaken for the accent colour'
Assert-True (-not $verdict.Passed) 'the white page with a title bar is rejected'

# ---------------------------------------------------------------------------
# 4. the REAL fallback composition: the shell, with an error card where the
#    route content should be.
# ---------------------------------------------------------------------------
Write-Case 'the real fallback composition is rejected'
$errorScreen = [GitGitSyntheticCapture]::ErrorFallback($width, $height, $sidebarWidth, $headerHeight,
                                                       $accentR, $accentG, $accentB)
$measurement = Measure-Buffer -Pixels $errorScreen
$verdict = Verdict -Measurement $measurement
# [FACT] The shell alone supplies accent pixels, so the WHOLE-WINDOW
# measurement would have accepted this screen: that is the defect the
# region-scoped measurement fixes.
$wholeWindow = Measure-WholeWindow -Pixels $errorScreen
Assert-True ($wholeWindow.AccentPixels -ge 150) (
    "the whole window holds accent pixels from the shell (" + $wholeWindow.AccentPixels +
    '), so a whole-window measurement would have accepted this screen')
Assert-Equal 0 $measurement.AccentPixels (
    'the route content area of the error composition holds NO accent pixels')
Assert-True (-not $verdict.Passed) 'the error composition is rejected'

# ---------------------------------------------------------------------------
# 5. the product, rendered
# ---------------------------------------------------------------------------
Write-Case 'the product itself is accepted'
$product = [GitGitSyntheticCapture]::Product($width, $height, $sidebarWidth, $headerHeight,
                                             $accentR, $accentG, $accentB, $true)
$measurement = Measure-Buffer -Pixels $product
$verdict = Verdict -Measurement $measurement
Assert-True ($measurement.AccentPixels -ge 150) ("the product holds accent pixels (" + $measurement.AccentPixels + ')')
Assert-True ($measurement.ColourRegions -ge 12) ("the product holds colour regions (" + $measurement.ColourRegions + ')')
Assert-True ($verdict.Passed) 'the rendered product is accepted'

# ---------------------------------------------------------------------------
# 6. the product without its accent-coloured control
# ---------------------------------------------------------------------------
Write-Case 'a layout without the product accent is rejected'
$noAccent = [GitGitSyntheticCapture]::Product($width, $height, $sidebarWidth, $headerHeight,
                                             $accentR, $accentG, $accentB, $false)
$measurement = Measure-Buffer -Pixels $noAccent
$verdict = Verdict -Measurement $measurement
Assert-True ($measurement.ColourRegions -ge 12) 'the layout still holds colour regions'
Assert-True (-not $verdict.Passed) 'a layout without the accent colour is rejected'

Write-Host ""
if ($script:Failures -gt 0) {
    Write-Host ("{0} of {1} cases FAILED" -f $script:Failures, $script:CaseCount) -ForegroundColor Red
    exit 1
}
Write-Host ("all {0} cases passed" -f $script:CaseCount) -ForegroundColor Green
exit 0
