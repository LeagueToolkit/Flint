# Flint Installer Assets

This folder contains custom branding images for the NSIS installer.

## Required Images

Replace the placeholder files with your custom Flint-branded images:

### 1. **installer-sidebar.bmp** (Left Side Panel)
- **Resolution**: 164 x 314 pixels
- **Format**: BMP (24-bit)
- **Usage**: Displayed on the left side of the installer during all steps
- **Design Tips**:
  - Use dark background (#1a1a1a or similar)
  - Include Flint logo/branding vertically centered
  - Keep important content in the center (top/bottom may be cut off)

### 2. **installer-header.bmp** (Top Banner)
- **Resolution**: 150 x 57 pixels
- **Format**: BMP (24-bit)
- **Usage**: Displayed as the header banner at the top of installer pages
- **Design Tips**:
  - Use dark background to match sidebar
  - Small Flint logo or text
  - Keep text/logo on the left side (header image is right-aligned)

### 3. **Icon** (Already exists)
- **File**: `icons/icon.ico`
- **Usage**: Installer icon, uninstaller icon, shortcuts
- **Note**: This is already configured and uses your current Flint icon

## Current Status

🔴 **PLACEHOLDER FILES** - Replace these with your custom images before building the installer!

## How to Create BMP Files

### Using Photoshop/GIMP:
1. Create new image with exact dimensions above
2. Design your installer graphics (dark theme recommended)
3. Save as **BMP** format
4. Select **24-bit** color depth
5. Save to this folder with exact filenames

### Using Online Tools:
- [Photopea](https://www.photopea.com/) - Free online Photoshop alternative
- [Paint.NET](https://www.getpaint.net/) - Free Windows image editor

## Color Palette (Dark Theme)

- **Background**: #1a1a1a (dark gray)
- **Text**: #e0e0e0 (light gray)
- **Accent**: Your Flint brand color
- **Logo**: Use your Flint logo/icon

## Testing

After adding your custom images:
1. Build the installer: `npm run tauri build`
2. Run the installer to preview your branding
3. Adjust images if needed and rebuild

## Notes

- BMP format is required by NSIS (PNG/JPG won't work)
- Images MUST be exactly the specified dimensions
- Use 24-bit color depth (no transparency needed)
- Dark theme colors are configured in `nsis/installer.nsi`
