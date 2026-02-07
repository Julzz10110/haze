// Copyright HAZE Blockchain. Plugin for Unreal Engine.

using UnrealBuildTool;

public class HazeBlockchain : ModuleRules
{
	public HazeBlockchain(ReadOnlyTargetRules Target) : base(Target)
	{
		PCHUsage = PCHUsageMode.UseExplicitOrSharedPCHs;
		CppStandard = CppStandard.Cpp17;

		PublicDependencyModuleNames.AddRange(new string[]
		{
			"Core",
			"HTTP",
			"Json",
			"JsonUtilities"
		});

		PrivateDependencyModuleNames.AddRange(new string[]
		{
			"CoreUObject",
			"Engine"
		});

		// Ed25519: optional ThirdParty. If ThirdParty/ed25519 exists with lib, link it.
		string ThirdPartyPath = System.IO.Path.GetFullPath(System.IO.Path.Combine(ModuleDirectory, "../../ThirdParty"));
		string Ed25519Path = System.IO.Path.Combine(ThirdPartyPath, "ed25519");
		if (System.IO.Directory.Exists(Ed25519Path))
		{
			PublicIncludePaths.Add(Ed25519Path);
			string LibName = (Target.Platform == UnrealTargetPlatform.Win64) ? "ed25519.lib" : "libed25519.a";
			string LibPath = System.IO.Path.Combine(Ed25519Path, LibName);
			if (System.IO.File.Exists(LibPath))
			{
				PublicAdditionalLibraries.Add(LibPath);
				PublicDefinitions.Add("HAZE_HAS_ED25519=1");
			}
		}
	}
}
