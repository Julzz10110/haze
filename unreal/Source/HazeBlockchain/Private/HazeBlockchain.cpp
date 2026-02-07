// Copyright HAZE Blockchain.

#include "HazeBlockchain.h"
#include "Misc/Paths.h"
#include "HAL/PlatformProcess.h"

#define LOCTEXT_NAMESPACE "FHazeBlockchainModule"

void FHazeBlockchainModule::StartupModule()
{
}

void FHazeBlockchainModule::ShutdownModule()
{
}

#undef LOCTEXT_NAMESPACE
IMPLEMENT_MODULE(FHazeBlockchainModule, HazeBlockchain)
