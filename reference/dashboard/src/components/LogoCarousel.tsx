import { FC } from 'react';

const LogoCarousel: FC = () => {
  // Company logos - using local logo files
  const companies = [
    { name: 'Jupiter', logo: '/logos/logo-jupiter-1.svg' },
    { name: 'Helius', logo: '/logos/helius.svg' },
    { name: 'Ottersec', logo: '/logos/ottersec.png' },
    { name: 'Solana', logo: '/logos/logo-solana-1.svg' },
  ];

  return (
    <div className="py-6">
      {/* Static logo display */}
      <div className="flex gap-12 sm:gap-16 items-center justify-center flex-wrap">
        {companies.map((company, index) => (
          <div
            key={`${company.name}-${index}`}
            className="flex items-center justify-center min-w-[180px] opacity-90 hover:opacity-100 transition-opacity"
          >
            <img
              src={company.logo}
              alt={company.name}
              className={`w-auto object-contain ${
                company.name === 'Jupiter' || company.name === 'Solana'
                  ? 'h-20 brightness-150 contrast-110'
                  : company.name === 'Helius'
                    ? 'h-8 brightness-150 contrast-110'
                  : company.name === 'Ottersec'
                    ? 'h-16 brightness-0 invert'
                    : 'h-16 brightness-150 contrast-110'
              }`}
              loading="lazy"
            />
          </div>
        ))}
      </div>
    </div>
  );
};

export default LogoCarousel;
