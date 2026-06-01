import { useMarketingAmbientScroll } from "../../hooks/useMarketingAmbientScroll";

const MarketingAmbientBackground = () => {
	const style = useMarketingAmbientScroll(true);

	return (
		<div className="marketing-ambient" style={style} aria-hidden>
			<div className="marketing-ambient__blob marketing-ambient__blob--a" />
			<div className="marketing-ambient__blob marketing-ambient__blob--b" />
			<div className="marketing-ambient__blob marketing-ambient__blob--c" />
		</div>
	);
};

export default MarketingAmbientBackground;
