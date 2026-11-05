#requires -Version 7
<#
.SYNOPSIS
    Capture a running window's pixels and decide whether they show the product.

.DESCRIPTION
    WHY THIS IS A SEPARATE FILE
    ===========================
    `verify-msi-launch.ps1` proves that the PACKAGED application starts: the
    installed executable presents a top-level window, and the application's own
    log says the webview booted. Both of those are true of a window that shows
    NOTHING: `MainWindowHandle` is non-zero for a window that was created and
    never drawn, and the boot marker only says that the webview's JavaScript
    ran.

    This file is the half that looks at the PIXELS. It captures the window and
    decides whether what was captured is the product, and it lives apart from
    the script that launches the application because the decision is the part
    that has to be testable on its own: `test-window-capture.ps1` exercises it
    with synthetic pixel buffers on any platform, including the Linux runner
    of the always-on `gate` workflow.

    WHAT "THE PRODUCT" MEANS HERE
    =============================
    A check that says "the capture is not one flat colour" is satisfied by a
    white page with a title bar, by a splash screen, by an error screen and by
    a Windows crash dialog. All four are exactly the failures this check
    exists to catch, so the criterion here is deliberately PRODUCT-SPECIFIC:

      * the capture must contain the product's own ACCENT COLOUR, and
      * the capture must contain more than one COLOUR REGION, so that a flat
        fill (a splash screen that happens to be the brand colour) cannot
        satisfy it.

    The accent colour is not written down here. It is READ OUT of the
    product's own `tailwind.config.js`, so the check cannot drift away from
    the palette the application actually ships. Only the CHROMATIC steps of
    that scale are used, and the reason is measured rather than guessed:

    [FACT] Measured 2026-11-04 on a blank Win32 window captured with
    `PrintWindow(PW_RENDERFULLCONTENT)`: the Windows 11 title bar paints
    `#F3F3FF`, and the product's palest accent step, `#f0fdf4`, is within
    12 per channel of it — so the pale steps match the native chrome of a
    window that contains no product at all. Measured on that same blank
    window: 39713 pixels matched `#f0fdf4`, 0 matched `#22c55e`, `#16a34a` or
    `#15803d`. The chromatic steps are the ones that mean something.

    THE CAPTURE
    ============
    `PrintWindow` against the window handle is used, because it captures the
    window even when something else is on top of it. WebView2 renders
    through DirectComposition, so the capture is asked for with
    `PW_RENDERFULLCONTENT` (the DWM path); the client area alone
    (`PW_CLIENTONLY`) is preferred where it works, because the native title
    bar is Windows' chrome and not the product.

    [FACT] A capture that comes back entirely black is a capture that did
    NOT work, not a blank product, so the strategies are tried in order and
    the first one that returns a non-black frame is kept. A genuinely blank
    surface comes back white, not black, and is reported as blank.

.NOTES
    No dependency is added. `System.Drawing` (which ships inside PowerShell's
    own directory, `System.Drawing.Common.dll`) is used to write the PNG
    artefact; the capture and the pixel statistics are plain Win32 GDI plus
    `Add-Type`.
#>

# ---------------------------------------------------------------------------
# The native half: GDI capture plus the pixel statistics.
# ---------------------------------------------------------------------------
# [FACT] The bitmap header has to declare `biPlanes` and `biBitCount` as
# 16-bit fields, because the native BITMAPINFOHEADER is 40 bytes and not 44.
# Declaring them as `int` makes the struct the wrong size, `CreateDIBSection`
# returns NULL, and the capture silently produces nothing at all.
$script:WindowCaptureNativeSource = @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

public sealed class GitGitCaptureResult
{
    public bool Ok;
    public int Width;
    public int Height;
    public string Error;
    public string Strategy;
    /// <summary>4 bytes per pixel, BGRA order, top-down.</summary>
    public byte[] Pixels;
}

public sealed class GitGitPixelStats
{
    public int Width;
    public int Height;
    public int TotalPixels;
    /// <summary>How many exactly different RGB values the capture contains.</summary>
    public int DistinctColours;
    /// <summary>How many quantised (4 bits per channel) colour regions it contains.</summary>
    public int ColourRegions;
    public int AccentPixels;
    public int[] AccentPixelsPerColour;
    public int[] TopColours;
    public int[] TopCounts;
    public bool AllBlack;
}

public static class GitGitWindowCapture
{
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }

    [StructLayout(LayoutKind.Sequential)]
    public struct BITMAPINFOHEADER
    {
        public int biSize;
        public int biWidth;
        public int biHeight;
        public short biPlanes;
        public short biBitCount;
        public int biCompression, biSizeImage, biXPelsPerMeter, biYPelsPerMeter, biClrUsed, biClrImportant;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct BITMAPINFO
    {
        public BITMAPINFOHEADER bmiHeader;
        public int bmiColors0, bmiColors1, bmiColors2;
    }

    [DllImport("user32.dll", SetLastError = true)]
    static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll", SetLastError = true)]
    static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);
    [DllImport("user32.dll")]
    static extern IntPtr GetDC(IntPtr hWnd);
    [DllImport("user32.dll")]
    static extern int ReleaseDC(IntPtr hWnd, IntPtr hDC);
    [DllImport("gdi32.dll")]
    static extern IntPtr CreateCompatibleDC(IntPtr hdc);
    [DllImport("gdi32.dll")]
    static extern bool DeleteDC(IntPtr hdc);
    [DllImport("gdi32.dll")]
    static extern IntPtr SelectObject(IntPtr hdc, IntPtr hgdiobj);
    [DllImport("gdi32.dll")]
    static extern bool DeleteObject(IntPtr hObject);
    [DllImport("gdi32.dll", SetLastError = true)]
    static extern IntPtr CreateDIBSection(IntPtr hdc, ref BITMAPINFO pbmi, uint usage,
                                          out IntPtr ppvBits, IntPtr hSection, uint dwOffset);
    [DllImport("gdi32.dll", SetLastError = true)]
    static extern bool BitBlt(IntPtr hdcDest, int xDest, int yDest, int w, int h,
                              IntPtr hdcSrc, int xSrc, int ySrc, uint rop);
    [DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
    static extern IntPtr CreateWindowEx(int dwExStyle, string lpClassName, string lpWindowName,
                                        uint dwStyle, int x, int y, int nWidth, int nHeight,
                                        IntPtr hWndParent, IntPtr hMenu, IntPtr hInstance, IntPtr lpParam);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode)]
    static extern IntPtr GetModuleHandle(string lpModuleName);
    [DllImport("user32.dll")]
    static extern bool UpdateWindow(IntPtr hWnd);
    [DllImport("user32.dll")]
    static extern bool DestroyWindow(IntPtr hWnd);
    [DllImport("user32.dll")]
    static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);

    const uint PW_CLIENTONLY = 0x00000001;
    const uint PW_RENDERFULLCONTENT = 0x00000002;
    const uint SRCCOPY = 0x00CC0020;
    const uint WS_POPUP = 0x80000000;
    const uint WS_VISIBLE = 0x10000000;

    static byte[] Grab(long hwnd, uint flags, bool screenCopy, out int width, out int height, out string error)
    {
        width = 0; height = 0; error = null;
        IntPtr h = new IntPtr(hwnd);
        RECT r;
        if (!GetWindowRect(h, out r))
        {
            error = "GetWindowRect failed (win32 error " + Marshal.GetLastWin32Error() + ")";
            return null;
        }
        int w = r.Right - r.Left;
        int hgt = r.Bottom - r.Top;
        if (w <= 0 || hgt <= 0)
        {
            error = "the window rectangle is empty (" + w + "x" + hgt + ")";
            return null;
        }

        IntPtr screenDc = GetDC(IntPtr.Zero);
        IntPtr memDc = CreateCompatibleDC(screenDc);
        if (memDc == IntPtr.Zero)
        {
            ReleaseDC(IntPtr.Zero, screenDc);
            error = "CreateCompatibleDC failed (win32 error " + Marshal.GetLastWin32Error() + ")";
            return null;
        }

        BITMAPINFO bi = new BITMAPINFO();
        bi.bmiHeader.biSize = 40;
        bi.bmiHeader.biWidth = w;
        bi.bmiHeader.biHeight = -hgt;   // top-down
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        bi.bmiHeader.biCompression = 0; // BI_RGB
        IntPtr bits;
        IntPtr dib = CreateDIBSection(memDc, ref bi, 0, out bits, IntPtr.Zero, 0);
        if (dib == IntPtr.Zero || bits == IntPtr.Zero)
        {
            DeleteDC(memDc);
            ReleaseDC(IntPtr.Zero, screenDc);
            error = "CreateDIBSection failed (win32 error " + Marshal.GetLastWin32Error() + ")";
            return null;
        }
        IntPtr old = SelectObject(memDc, dib);
        try
        {
            if (screenCopy)
            {
                BitBlt(memDc, 0, 0, w, hgt, screenDc, r.Left, r.Top, SRCCOPY);
            }
            else
            {
                PrintWindow(h, memDc, flags);
            }
            byte[] raw = new byte[w * 4 * hgt];
            Marshal.Copy(bits, raw, 0, raw.Length);
            // GDI hands the pixels over in BGRA order, and GDI+'s
            // `Format32bppArgb` — which the PNG artefact is written through —
            // keeps them in BGRA order in memory too, so the buffer is passed
            // on untouched. Only the ALPHA byte is replaced: GDI leaves it
            // undefined for the content it draws, and an undefined alpha byte
            // is what turns the saved PNG transparent.
            byte[] bgra = new byte[raw.Length];
            for (int i = 0; i < raw.Length; i += 4)
            {
                bgra[i] = raw[i];
                bgra[i + 1] = raw[i + 1];
                bgra[i + 2] = raw[i + 2];
                bgra[i + 3] = 255;
            }
            width = w; height = hgt;
            return bgra;
        }
        finally
        {
            SelectObject(memDc, old);
            DeleteObject(dib);
            DeleteDC(memDc);
            ReleaseDC(IntPtr.Zero, screenDc);
        }
    }

    /// <summary>Capture the window with one PrintWindow flag set.</summary>
    public static GitGitCaptureResult Capture(long hwnd, uint flags)
    {
        GitGitCaptureResult res = new GitGitCaptureResult();
        int w, h; string err;
        res.Pixels = Grab(hwnd, flags, false, out w, out h, out err);
        res.Width = w; res.Height = h;
        res.Error = err;
        res.Strategy = "PrintWindow(0x" + flags.ToString("X") + ")";
        res.Ok = res.Pixels != null;
        return res;
    }

    /// <summary>Copy the window's rectangle off the screen.</summary>
    public static GitGitCaptureResult CaptureScreen(long hwnd)
    {
        GitGitCaptureResult res = new GitGitCaptureResult();
        int w, h; string err;
        res.Pixels = Grab(hwnd, 0, true, out w, out h, out err);
        res.Width = w; res.Height = h;
        res.Error = err;
        res.Strategy = "BitBlt(screen)";
        res.Ok = res.Pixels != null;
        return res;
    }

    /// <summary>
    /// A deliberately blank surface: a real top-level window with no content
    /// in it. The handle is returned so that it can be captured through
    /// EXACTLY the same path as the product's window.
    /// </summary>
    public static IntPtr CreateBlankSurface(int width, int height)
    {
        IntPtr instance = GetModuleHandle(null);
        IntPtr win = CreateWindowEx(0, "STATIC", "gitgit blank control",
                                    (WS_POPUP | WS_VISIBLE), 0, 0, width, height,
                                    IntPtr.Zero, IntPtr.Zero, instance, IntPtr.Zero);
        if (win == IntPtr.Zero) return IntPtr.Zero;
        ShowWindow(win, 5 /* SW_SHOW */);
        UpdateWindow(win);
        System.Threading.Thread.Sleep(250);
        return win;
    }

    /// <summary>Destroy a window made by CreateBlankSurface.</summary>
    public static void DestroyBlankSurface(IntPtr win)
    {
        if (win != IntPtr.Zero) DestroyWindow(win);
    }

    /// <summary>The pixel statistics the decision is made from.</summary>
    public static GitGitPixelStats Measure(byte[] argb, int width, int height, int[] accentRgb, int tolerance)
    {
        GitGitPixelStats s = new GitGitPixelStats();
        s.Width = width; s.Height = height;
        s.TotalPixels = width * height;
        s.AccentPixelsPerColour = new int[accentRgb.Length / 3];
        s.TopColours = new int[0];
        s.TopCounts = new int[0];
        if (argb == null || width <= 0 || height <= 0) return s;

        HashSet<int> exact = new HashSet<int>();
        int[] regions = new int[4096];
        int regionCount = 0;
        int accent = 0;
        bool allBlack = true;
        int accentCount = accentRgb.Length / 3;

        for (int i = 0; i + 3 < argb.Length; i += 4)
        {
            // The buffer is BGRA — GDI's own order, and the order GDI+'s
            // `Format32bppArgb` keeps in memory, so the PNG artefact and this
            // measurement read the same bytes.
            int b = argb[i];
            int g = argb[i + 1];
            int r = argb[i + 2];
            if (r != 0 || g != 0 || b != 0) allBlack = false;
            exact.Add((r << 16) | (g << 8) | b);
            int q = ((r >> 4) << 8) | ((g >> 4) << 4) | (b >> 4);
            if (regions[q] == 0) regionCount++;
            regions[q]++;
            for (int a = 0; a < accentCount; a++)
            {
                int ar = accentRgb[a * 3];
                int ag = accentRgb[a * 3 + 1];
                int ab = accentRgb[a * 3 + 2];
                if (Math.Abs(r - ar) <= tolerance && Math.Abs(g - ag) <= tolerance && Math.Abs(b - ab) <= tolerance)
                {
                    accent++;
                    s.AccentPixelsPerColour[a]++;
                    break;
                }
            }
        }

        s.DistinctColours = exact.Count;
        s.ColourRegions = regionCount;
        s.AccentPixels = accent;
        s.AllBlack = allBlack;

        // The eight most common quantised colours, for a human reading the log.
        List<int> keys = new List<int>();
        for (int i = 0; i < regions.Length; i++) if (regions[i] > 0) keys.Add(i);
        keys.Sort((x, y) => regions[y].CompareTo(regions[x]));
        int take = Math.Min(8, keys.Count);
        s.TopColours = new int[take];
        s.TopCounts = new int[take];
        for (int i = 0; i < take; i++)
        {
            int q = keys[i];
            int r = ((q >> 8) & 0xF) * 17;
            int g = ((q >> 4) & 0xF) * 17;
            int b = (q & 0xF) * 17;
            s.TopColours[i] = (255 << 24) | (r << 16) | (g << 8) | b;
            s.TopCounts[i] = regions[q];
        }
        return s;
    }
}
'@
function Initialize-WindowCaptureNative {
    <#
    .SYNOPSIS
        Compile the native capture helper, once per PowerShell process.
    #>
    if ('GitGitWindowCapture' -as [type]) { return }
    Add-Type -TypeDefinition $script:WindowCaptureNativeSource -ErrorAction Stop
}

# ---------------------------------------------------------------------------
# The product's own palette.
# ---------------------------------------------------------------------------
function Get-ProductAccentColors {
    <#
    .SYNOPSIS
        The chromatic steps of the product's accent scale, read out of the
        product's own Tailwind configuration.

    .DESCRIPTION
        [FACT] The accent scale is read from `tailwind.config.js` rather
        than written down, so a change to the palette cannot leave the check
        asserting a colour the product no longer uses.

        [FACT] Only the CHROMATIC steps are kept. The product's palest step
        `#f0fdf4` is within 12 per channel of the `#F3F3FF` that the Windows
        11 title bar paints, so a window containing no product at all matches
        it (measured: 39713 matching pixels on a blank window). The steps
        kept here are `#22c55e`, `#16a34a` and `#15803d`, none of which the
        native chrome can produce.

    .PARAMETER TailwindConfigPath
        Path to `tailwind.config.js`.

    .PARAMETER MinSaturation
        The smallest saturation a step may have and still be used. Default
        0.30 — `#f0fdf4` has 0.05 and `#dcfce7` has 0.13, while `#22c55e`
        has 0.83.

    .PARAMETER Tolerance
        How far a pixel may sit from an accent step, per channel. Default 12.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string]$TailwindConfigPath,
        [double]$MinSaturation = 0.30,
        [int]$Tolerance = 12
    )

    if (-not (Test-Path -LiteralPath $TailwindConfigPath)) {
        throw "the product's Tailwind configuration '$TailwindConfigPath' is not there, so the accent colour is unknown"
    }
    $text = Get-Content -LiteralPath $TailwindConfigPath -Raw -Encoding UTF8

    $start = $text.IndexOf('accent')
    if ($start -lt 0) {
        throw "there is no 'accent' palette in '$TailwindConfigPath', so the accent colour is unknown"
    }
    $open = $text.IndexOf('{', $start)
    if ($open -lt 0) {
        throw "the 'accent' palette in '$TailwindConfigPath' has no block, so the accent colour is unknown"
    }
    $depth = 0
    $close = -1
    for ($i = $open; $i -lt $text.Length; $i++) {
        if ($text[$i] -eq '{') { $depth++ }
        elseif ($text[$i] -eq '}') {
            $depth--
            if ($depth -eq 0) { $close = $i; break }
        }
    }
    if ($close -lt 0) {
        throw "the 'accent' palette in '$TailwindConfigPath' is not closed, so the accent colour is unknown"
    }
    $block = $text.Substring($open, $close - $open + 1)

    $matches = [regex]::Matches($block, '(?im)^\s*(\d+)\s*:\s*[''"]?(?<hex>#[0-9a-f]{6})[''"]?')
    if ($matches.Count -eq 0) {
        throw "the 'accent' palette in '$TailwindConfigPath' carries no colours, so the accent colour is unknown"
    }

    $colors = @()
    foreach ($m in $matches) {
        $hex = $m.Groups['hex'].Value
        $r = [Convert]::ToInt32($hex.Substring(1, 2), 16)
        $g = [Convert]::ToInt32($hex.Substring(3, 2), 16)
        $b = [Convert]::ToInt32($hex.Substring(5, 2), 16)
        $max = [Math]::Max($r, [Math]::Max($g, $b))
        $min = [Math]::Min($r, [Math]::Min($g, $b))
        $saturation = if ($max -eq 0) { 0.0 } else { ($max - $min) / $max }
        if ($saturation -lt $MinSaturation) { continue }
        $colors += [pscustomobject]@{
            Step       = $m.Groups[1].Value
            Hex        = $hex
            R          = $r
            G          = $g
            B          = $b
            Saturation = [Math]::Round($saturation, 3)
        }
    }
    if ($colors.Count -eq 0) {
        throw ("the 'accent' palette in '$TailwindConfigPath' carries no chromatic colour, " +
               "so there is nothing a capture could be checked against")
    }
    return $colors
}

# ---------------------------------------------------------------------------
# The capture.
# ---------------------------------------------------------------------------
function Capture-WindowPixels {
    <#
    .SYNOPSIS
        Capture a window's pixels with every strategy, and return them all.

    .DESCRIPTION
        [FACT] Every strategy is tried and every frame is kept, because
        which one carries the product is a property of the machine the
        capture runs on and not something to be assumed. Measured on the
        packaged application (Windows 11, WebView2 through DirectComposition,
        2026-11-04):

            PrintWindow(PW_CLIENTONLY | PW_RENDERFULLCONTENT)  173 colour regions, 6753 accent pixels
            PrintWindow(PW_RENDERFULLCONTENT)                 203 colour regions, 6753 accent pixels
            PrintWindow(PW_CLIENTONLY)                          2 colour regions,    0 accent pixels
            PrintWindow(0)                                    111 colour regions,    0 accent pixels

        `PW_CLIENTONLY` on its own does NOT carry the webview: WebView2
        renders through DirectComposition, and only the DWM path
        (`PW_RENDERFULLCONTENT`) asks the compositor for what is on screen.
        That is why the DWM strategies come first.

        The strategies, in order:
          1. `PrintWindow(PW_CLIENTONLY | PW_RENDERFULLCONTENT)`
          2. `PrintWindow(PW_RENDERFULLCONTENT)`
          3. `PrintWindow(PW_CLIENTONLY)`
          4. `PrintWindow(0)`
          5. A screen copy of the window's rectangle — used ONLY when every
             `PrintWindow` came back entirely black, because a screen copy
             shows whatever happens to be on top of the window.

    .PARAMETER Handle
        The window handle.

    .PARAMETER Strategies
        Which `PrintWindow` flag sets to try. Default: all of them, in the
        order above.

    .OUTPUTS
        The candidate captures, each with `Strategy`, `Width`, `Height`,
        `Pixels` (BGRA bytes), `AllBlack` and `DistinctColours`.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][IntPtr]$Handle,
        [int[]]$Strategies = @(3, 2, 1, 0)
    )

    Initialize-WindowCaptureNative

    $candidates = @()
    foreach ($flags in $Strategies) {
        $result = [GitGitWindowCapture]::Capture($Handle.ToInt64(), [uint32]$flags)
        if (-not $result.Ok) {
            $candidates += [pscustomobject]@{
                Strategy = $result.Strategy; Ok = $false; Width = 0; Height = 0
                Pixels = $null; AllBlack = $true; DistinctColours = 0; Error = $result.Error
            }
            continue
        }
        $stats = [GitGitWindowCapture]::Measure($result.Pixels, $result.Width, $result.Height, [int[]]@(), 0)
        $candidates += [pscustomobject]@{
            Strategy        = $result.Strategy
            Ok              = $true
            Width           = $result.Width
            Height          = $result.Height
            Pixels          = $result.Pixels
            AllBlack        = $stats.AllBlack
            DistinctColours = $stats.DistinctColours
            Error           = $null
        }
    }

    # A screen copy is the last resort, and only when the `PrintWindow`
    # path produced nothing at all: an entirely black frame is what a
    # capture that did NOT work looks like.
    if (@($candidates | Where-Object { -not $_.AllBlack }).Count -eq 0) {
        $screen = [GitGitWindowCapture]::CaptureScreen($Handle.ToInt64())
        if ($screen.Ok) {
            $stats = [GitGitWindowCapture]::Measure($screen.Pixels, $screen.Width, $screen.Height, [int[]]@(), 0)
            $candidates += [pscustomobject]@{
                Strategy = $screen.Strategy; Ok = $true; Width = $screen.Width; Height = $screen.Height
                Pixels = $screen.Pixels; AllBlack = $stats.AllBlack
                DistinctColours = $stats.DistinctColours; Error = $null
            }
        }
    }

    return $candidates
}

function Get-WindowCaptureEvidence {
    <#
    .SYNOPSIS
        Capture a window, measure the capture, and decide whether it shows
        the product.

    .DESCRIPTION
        [FACT] The candidate that is kept is the one with the MOST accent
        pixels, with ties going to the strategy that was tried first. That
        is a choice about which frame carries the product, not about whether
        the product is there: the decision that follows still applies the
        full threshold, so a window with no accent colour in it at all is
        rejected whichever frame was kept.

        [FACT] A screen copy is never preferred over a `PrintWindow` frame.
        A screen copy shows whatever happens to be on top of the window, so
        it can only be trusted when `PrintWindow` produced nothing.

    .PARAMETER Handle
        The window handle.

    .PARAMETER AccentColors
        The accent colours, as returned by `Get-ProductAccentColors`.

    .PARAMETER Tolerance
        How far a pixel may sit from an accent colour, per channel.

    .PARAMETER MinAccentPixels
        How many accent pixels the capture must hold. Default 150.

    .PARAMETER MinColourRegions
        How many quantised colour regions the capture must hold. Default 12.

    .OUTPUTS
        `Capture` (the frame that was kept), `Measurement`, `Verdict` and
        `Candidates` (one line per strategy tried).
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][IntPtr]$Handle,
        [Parameter(Mandatory)][object[]]$AccentColors,
        [int]$Tolerance = 12,
        [int]$MinAccentPixels = 150,
        [int]$MinColourRegions = 12
    )

    $candidates = @(Capture-WindowPixels -Handle $Handle)
    $best = $null
    $bestAccent = -1
    foreach ($candidate in $candidates) {
        if (-not $candidate.Ok) { continue }
        $stats = Measure-CapturedPixels -Pixels $candidate.Pixels -Width $candidate.Width `
            -Height $candidate.Height -AccentColors $AccentColors -Tolerance $Tolerance
        $candidate | Add-Member -NotePropertyName AccentPixels -NotePropertyValue $stats.AccentPixels -Force
        $candidate | Add-Member -NotePropertyName ColourRegions -NotePropertyValue $stats.ColourRegions -Force
        if ($stats.AccentPixels -gt $bestAccent) {
            $bestAccent = $stats.AccentPixels
            $best = $candidate
            $bestMeasurement = $stats
        }
    }

    if ($null -eq $best) {
        return [pscustomobject]@{
            Capture    = $null
            Measurement = $null
            Verdict    = [pscustomobject]@{
                Passed   = $false
                Reason   = 'the window could not be captured at all'
                Expected = 'a capture of the window'
                Measured = 'every capture strategy failed'
            }
            Candidates = $candidates
        }
    }

    $verdict = Test-CapturedProduct -Measurement $bestMeasurement `
        -MinAccentPixels $MinAccentPixels -MinColourRegions $MinColourRegions
    return [pscustomobject]@{
        Capture    = $best
        Measurement = $bestMeasurement
        Verdict    = $verdict
        Candidates = $candidates
    }
}

function Get-BlankControlEvidence {
    <#
    .SYNOPSIS
        The negative control: a deliberately blank window, captured and
        decided through the SAME functions as the product's window.

    .DESCRIPTION
        [FACT] This is the control that makes the rest of the check mean
        something. It creates a real top-level window with no content in it,
        captures it through `Capture-WindowPixels`, and decides on it with
        `Test-CapturedProduct` — the very same code that the application's
        own window goes through. The only thing that differs is what is in
        the window.

        Measured 2026-11-04, at 1280x800: 2 distinct colours, 2 colour
        regions, 0 accent pixels — and the decision REJECTS it.

        The window is the built-in `STATIC` class, so no window class has to
        be registered and no dependency is added.
    #>
    [CmdletBinding()]
    param(
        [int]$Width = 1280,
        [int]$Height = 800,
        [Parameter(Mandatory)][object[]]$AccentColors,
        [int]$Tolerance = 12
    )

    Initialize-WindowCaptureNative
    $handle = [GitGitWindowCapture]::CreateBlankSurface($Width, $Height)
    if ($handle -eq [IntPtr]::Zero) {
        return [pscustomobject]@{
            Capture    = $null
            Measurement = $null
            Verdict    = [pscustomobject]@{
                Passed   = $false
                Reason   = 'the blank control window could not be created'
                Expected = 'a blank control window'
                Measured = 'CreateBlankSurface returned no window'
            }
            Candidates = @()
        }
    }
    try {
        return Get-WindowCaptureEvidence -Handle ([IntPtr]$handle) `
            -AccentColors $AccentColors -Tolerance $Tolerance
    } finally {
        [GitGitWindowCapture]::DestroyBlankSurface($handle)
    }
}

function Save-CapturedPng {
    <#
    .SYNOPSIS
        Write a capture out as a PNG, so a human can look at it.

    .DESCRIPTION
        [FACT] `System.Drawing.Common.dll` ships inside PowerShell's own
        directory, so this adds no dependency. The pixels arrive in BGRA
        order, which is the order `Format32bppArgb` keeps in memory, so they
        are copied straight in.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][byte[]]$Pixels,
        [Parameter(Mandatory)][int]$Width,
        [Parameter(Mandatory)][int]$Height,
        [Parameter(Mandatory)][string]$Path
    )

    Add-Type -AssemblyName System.Drawing -ErrorAction Stop
    $bitmap = [System.Drawing.Bitmap]::new($Width, $Height, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    try {
        $data = $bitmap.LockBits(
            [System.Drawing.Rectangle]::new(0, 0, $Width, $Height),
            [System.Drawing.Imaging.ImageLockMode]::WriteOnly,
            [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        try {
            [System.Runtime.InteropServices.Marshal]::Copy($Pixels, 0, $data.Scan0, $Pixels.Length)
        } finally {
            $bitmap.UnlockBits($data)
        }
        $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $bitmap.Dispose()
    }
    return $Path
}

# ---------------------------------------------------------------------------
# The measurement and the decision.
# ---------------------------------------------------------------------------
function Measure-CapturedPixels {
    <#
    .SYNOPSIS
        Measure a capture: how many different colours it holds, and how many
        pixels are the product's accent colour.

    .PARAMETER Pixels
        The BGRA bytes of the capture.

    .PARAMETER AccentColors
        The accent colours, as returned by `Get-ProductAccentColors`.

    .PARAMETER Tolerance
        How far a pixel may sit from an accent colour, per channel.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][byte[]]$Pixels,
        [Parameter(Mandatory)][int]$Width,
        [Parameter(Mandatory)][int]$Height,
        [Parameter(Mandatory)][object[]]$AccentColors,
        [int]$Tolerance = 12
    )

    Initialize-WindowCaptureNative
    $flat = [int[]]::new($AccentColors.Count * 3)
    for ($i = 0; $i -lt $AccentColors.Count; $i++) {
        $flat[$i * 3] = $AccentColors[$i].R
        $flat[$i * 3 + 1] = $AccentColors[$i].G
        $flat[$i * 3 + 2] = $AccentColors[$i].B
    }
    return [GitGitWindowCapture]::Measure($Pixels, $Width, $Height, $flat, $Tolerance)
}

function Get-TopColourSummary {
    <#
    .SYNOPSIS
        The most common colours of a capture, as text a human can read.

    .DESCRIPTION
        The colours are quantised (four bits per channel), so the list shows
        the palette of the screen rather than the thousands of slightly
        different shades that antialiased text produces.
    #>
    [CmdletBinding()]
    param([Parameter(Mandatory)]$Measurement)

    $parts = @()
    for ($i = 0; $i -lt $Measurement.TopColours.Length; $i++) {
        $v = $Measurement.TopColours[$i]
        $r = ($v -shr 16) -band 0xFF
        $g = ($v -shr 8) -band 0xFF
        $b = $v -band 0xFF
        $parts += ('#{0:X2}{1:X2}{2:X2} x{3}' -f $r, $g, $b, $Measurement.TopCounts[$i])
    }
    return ($parts -join ', ')
}

function Test-CapturedProduct {
    <#
    .SYNOPSIS
        Decide whether a capture shows the product.

    .DESCRIPTION
        Two conditions, and BOTH have to hold:

          1. the capture contains at least `MinAccentPixels` pixels of the
             product's own accent colour, and
          2. the capture contains at least `MinColourRegions` quantised
             colour regions, so a flat fill cannot satisfy it.

        [FACT] Condition 1 alone would be satisfied by a splash screen that
        is the brand colour; condition 2 alone would be satisfied by a white
        page with a title bar. Together they are not.

    .PARAMETER Measurement
        What `Measure-CapturedPixels` returned.

    .PARAMETER MinAccentPixels
        How many accent pixels the capture must hold. Default 150.

    .PARAMETER MinColourRegions
        How many quantised colour regions the capture must hold. Default 12.

    .OUTPUTS
        A verdict: `Passed`, `Reason`, `Expected` and `Measured`, both of the
        last two being text a human can read.
    #>
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]$Measurement,
        [int]$MinAccentPixels = 150,
        [int]$MinColourRegions = 12
    )

    $expected = ("at least $MinAccentPixels pixels of the product's accent colour " +
                 "and at least $MinColourRegions colour regions")
    $measured = ("{0} accent pixels, {1} distinct colours, {2} colour regions, {3}x{4}" -f
                 $Measurement.AccentPixels, $Measurement.DistinctColours,
                 $Measurement.ColourRegions, $Measurement.Width, $Measurement.Height)

    if ($Measurement.AccentPixels -lt $MinAccentPixels) {
        return [pscustomobject]@{
            Passed   = $false
            Reason   = "the capture does not contain the product's accent colour"
            Expected = $expected
            Measured = $measured
        }
    }
    if ($Measurement.ColourRegions -lt $MinColourRegions) {
        return [pscustomobject]@{
            Passed   = $false
            Reason   = 'the capture is a flat fill: it has the accent colour but no layout'
            Expected = $expected
            Measured = $measured
        }
    }
    return [pscustomobject]@{
        Passed   = $true
        Reason   = 'the capture contains the product accent colour and more than one colour region'
        Expected = $expected
        Measured = $measured
    }
}
