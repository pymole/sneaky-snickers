package rules

import (
	"math/rand"
)

type WrappedSpiralRuleset struct {
	WrappedRuleset

	Seed int64
}

func (r *WrappedSpiralRuleset) Name() string { return "wrapped+spiral" }

func (r *WrappedSpiralRuleset) CreateNextBoardState(prevState *BoardState, moves []SnakeMove) (*BoardState, error) {
	nextBoardState, err := r.WrappedRuleset.CreateNextBoardState(prevState, moves)
	if err != nil {
		return nil, err
	}

	err = r.populateHazards(nextBoardState, prevState.Turn+1)
	if err != nil {
		return nil, err
	}

	return nextBoardState, nil
}

func (p *Point) rotateCW() {
	*p = Point { p.Y, -p.X }
}

func (p *Point) add(other Point) {
	p.X += other.X
	p.Y += other.Y
}

type SpiralState struct {
	Position Point
	Direction Point
	Amplitude int
	ArmLength int
}

func newSpiralState(spiralCenter Point) SpiralState {
	return SpiralState{
		Position: spiralCenter,
		Direction: Point{0, 1},
		Amplitude: 0,
		ArmLength: -1,
	}
}

func (s *SpiralState) nextPoint() Point {
	result := s.Position

	s.ArmLength += 1
	if s.ArmLength == s.Amplitude {
		s.ArmLength = 0
		if (Point{0, 1}) == s.Direction {
			s.Position.add(s.Direction)
			s.Direction.rotateCW()
			s.Amplitude += 2
		} else {
			s.Direction.rotateCW()
			s.Position.add(s.Direction)
		}
	} else {
		s.Position.add(s.Direction)
	}

	return result
}

func (r *WrappedSpiralRuleset) populateHazards(b *BoardState, turn int32) error {
	b.Hazards = []Point{}

	randGenerator := rand.New(rand.NewSource(r.Seed))
	spiralCenter := Point {
		X: int32(randGenerator.Intn(int(b.Width))),
		Y: int32(randGenerator.Intn(int(b.Height))),
	}
	spiralState := newSpiralState(spiralCenter)
	spiralSize := turn / 3

	for i := int32(0); i <= spiralSize; i++ {
		p := spiralState.nextPoint()
		if 0 <= p.X && p.X < b.Width && 0 <= p.Y && p.Y < b.Height {
			b.Hazards = append(b.Hazards, p)
		}
	}

	return nil
}
