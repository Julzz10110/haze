# Installation Guide

## Prerequisites

- Unity 2020.3 LTS or later
- .NET Standard 2.1 or .NET Framework 4.8

## Step 1: Install Chaos.NaCl

The SDK requires Chaos.NaCl for Ed25519 cryptography. You have two options:

### Option A: Download DLL

1. Download `Chaos.NaCl.dll` from [Chaos.NaCl releases](https://github.com/CodesInChaos/Chaos.NaCl/releases)
2. Place it in `Assets/Plugins/` folder
3. Ensure it's compatible with your Unity target platform (.NET Standard 2.1)

### Option B: NuGet (if using NuGet for Unity)

```bash
nuget install Chaos.NaCl
```

## Step 2: Install Newtonsoft.Json

Unity 2020.3+ includes Newtonsoft.Json via Package Manager:

1. Open Package Manager (Window → Package Manager)
2. Search for "Newtonsoft Json"
3. Install "com.unity.nuget.newtonsoft-json" (version 3.0.2 or later)

## Step 3: Install HAZE SDK

### Via Git URL (recommended)

1. Open Package Manager (Window → Package Manager)
2. Click "+" → "Add package from git URL"
3. Enter: `https://github.com/haze-blockchain/haze.git?path=unity`
4. Click "Add"

### Via OpenUPM

```bash
openupm add com.haze.blockchain
```

### Manual Installation

1. Clone or download this repository
2. Copy the `unity` folder to your project's `Packages` folder
3. Unity will automatically recognize it as a local package

## Step 4: Verify Installation

Create a test script:

```csharp
using Haze;
using Haze.Crypto;

public class TestInstallation : MonoBehaviour
{
    void Start()
    {
        var keyPair = KeyPair.Generate();
        Debug.Log($"Address: {keyPair.GetAddressHex()}");
    }
}
```

If you see an address printed, installation is successful!

## Troubleshooting

### "Chaos.NaCl not found"

- Ensure `Chaos.NaCl.dll` is in `Assets/Plugins/`
- Check that the DLL is compatible with your target platform
- Verify `.asmdef` references include Chaos.NaCl

### "Newtonsoft.Json not found"

- Install via Package Manager: `com.unity.nuget.newtonsoft-json`
- Or manually add Newtonsoft.Json.dll to `Assets/Plugins/`

### "Assembly definition errors"

- Check that `Haze.asmdef` references Newtonsoft.Json correctly
- Ensure GUID matches your Unity version's Newtonsoft.Json package
