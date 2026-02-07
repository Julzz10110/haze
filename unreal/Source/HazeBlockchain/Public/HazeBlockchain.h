// Copyright HAZE Blockchain. Unreal Engine plugin.

#pragma once

#include "CoreMinimal.h"
#include "Modules/ModuleManager.h"

class FHazeBlockchainModule : public IModuleInterface
{
public:
	virtual void StartupModule() override;
	virtual void ShutdownModule() override;
};
