namespace Haze.Mistborn
{
    /// <summary>
    /// Density level limits and documentation
    /// </summary>
    public static class DensityLimits
    {
        /// <summary>
        /// Maximum size for Ethereal density (5 KB)
        /// </summary>
        public const int EtherealMaxBytes = 5 * 1024;

        /// <summary>
        /// Maximum size for Light density (50 KB)
        /// </summary>
        public const int LightMaxBytes = 50 * 1024;

        /// <summary>
        /// Maximum size for Dense density (5 MB)
        /// </summary>
        public const int DenseMaxBytes = 5 * 1024 * 1024;

        /// <summary>
        /// Maximum size for Core density (50 MB+)
        /// </summary>
        public const int CoreMaxBytes = 50 * 1024 * 1024;

        /// <summary>
        /// Get maximum size for a density level
        /// </summary>
        public static int GetMaxSize(DensityLevel density)
        {
            return density switch
            {
                DensityLevel.Ethereal => EtherealMaxBytes,
                DensityLevel.Light => LightMaxBytes,
                DensityLevel.Dense => DenseMaxBytes,
                DensityLevel.Core => CoreMaxBytes,
                _ => EtherealMaxBytes
            };
        }

        /// <summary>
        /// Check if metadata size is within density limit
        /// </summary>
        public static bool IsWithinLimit(DensityLevel density, int metadataSizeBytes)
        {
            return metadataSizeBytes <= GetMaxSize(density);
        }

        /// <summary>
        /// Get recommended density level for given metadata size
        /// </summary>
        public static DensityLevel GetRecommendedDensity(int metadataSizeBytes)
        {
            if (metadataSizeBytes <= EtherealMaxBytes)
                return DensityLevel.Ethereal;
            if (metadataSizeBytes <= LightMaxBytes)
                return DensityLevel.Light;
            if (metadataSizeBytes <= DenseMaxBytes)
                return DensityLevel.Dense;
            return DensityLevel.Core;
        }
    }
}
