import React, { useState, useEffect } from 'react';
import Cookies from 'js-cookie';

const apiRequest = async (url, method, body) => {
  const response = await fetch(url, {
    method,
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(body),
  });
  return response.json();
};

const get = async (url) => apiRequest(url, 'GET');
const post = async (url, body) => apiRequest(url, 'POST', body);

const playerId = Cookies.get('web4_account_id');
const contractId = window._web4Config?.contractName;

const Dice = ({ value, size }) => (
  <div>
    D{size}: {value}
  </div>
);

const joinGame = async (gameId) => {
  await post(`/web4/contract/${contractId}/join_game`, { game_id: gameId });
  window.location.href = `/games/${gameId}`;
};

const GameList = ({ games }) => (
  <ul>
    {games.map(game => (
      <li key={game.id}>
        Game {game.id}: {game.players[0]} vs {game.players[1] || '???'} {
          game.players.find(p => p == playerId)
            ? <a href={`/games/${game.id}`}>Resume</a>
            : <button onClick={() => joinGame(game.id)}>Join</button>
        }
      </li>
    ))}
  </ul>
);

const OpenGamesList = () => {
  const [openGames, setOpenGames] = useState([]);

  useEffect(() => {
    const interval = setInterval(async () => {
      const games = await get('/api/games');
      setOpenGames(games);
    }, 2000);

    return () => clearInterval(interval);
  }, []);

  return (
    <div>
      <h2>Open games</h2>
      <GameList games={openGames} />
    </div>
  );
};

const AwaitingTurnGamesList = ({ gameId }) => {
  const [games, setGames] = useState([]);

  useEffect(() => {
    const interval = setInterval(async () => {
      const games = await get(`/api/users/${playerId}/games`);
      setGames(games.filter(game => game.id !== gameId && game.current_player === game.players.indexOf(playerId)));
    }, 2000);

    return () => clearInterval(interval);
  }, []);

  if (games.length === 0) {
    return null;
  }

  return (
    <div>
      <h2>Awaiting your turn</h2>
      <GameList games={games} />
    </div>
  );
};

const Game = ({ gameId }) => {
  const [gameState, setGameState] = useState(null);
  const [selectedDice, setSelectedDice] = useState([]); // To store indices of selected dice for an attack
  const [selectedDefenderDie, setSelectedDefenderDie] = useState(null); // Index of selected defender die

  // Fetch game status
  useEffect(() => {
    const interval = setInterval(async () => {
      const status = await get(`/api/games/${gameId}/status`);
      setGameState(status);
    }, 2000);

    return () => clearInterval(interval);
  }, [gameId]);

  const attack = async (attackerDieIndices, defenderDieIndex) => {
    await post(`/web4/contract/${contractId}/attack`, { game_id: gameId, attacker_die_indices: attackerDieIndices, defender_die_index: defenderDieIndex });
  };

  const selectDieForAttack = (index) => {
    setSelectedDice(prev => {
      // Add or remove the die index from the selection
      if (prev.includes(index)) {
        return prev.filter(i => i !== index);
      } else {
        return [...prev, index];
      }
    });
  };

  const selectDefenderDieForAttack = (index) => {
    setSelectedDefenderDie(index);
  };

  const performAttack = async () => {
    if (selectedDice.length === 0) {
      alert('Select at least one die to attack');
      return;
    }

    await attack(selectedDice, selectedDefenderDie);
    // Reset selection after attack
    setSelectedDice([]);
    setSelectedDefenderDie(null);
  };

  const renderDice = (playerDice, dicePlayerId, isActive, captured) => (
    <div>
      <h3>{dicePlayerId} {dicePlayerId == playerId && '(You)'}</h3>
      { isActive && dicePlayerId == playerId && <p>It's your turn</p> }
      <h4>Dice</h4>
      {playerDice.map((die, index) => {
        const isSelected = !isActive
          ? index === selectedDefenderDie
          : selectedDice.includes(index);
        return (
          <button
            key={index}
            onClick={() => !isActive ? selectDefenderDieForAttack(index) : selectDieForAttack(index)}
            style={{ backgroundColor: isSelected ? 'yellow' : 'white' }}
          >
            <Dice value={die.value} size={die.size} />
          </button>
        );
      })}
      <h4>Captured</h4>
      <p>{captured.length > 0 ? captured.map((die) => `D${die}`).join(', ') : 'None'}</p>
      <p>Score: {captured.reduce((a, b) => a + b, 0)}</p>
    </div>
  );

  if (!gameState) {
    return <div>Loading...</div>;
  }

  return (
    <div>
      <h2>{gameState.players[0]} playing against {gameState.players[1]}</h2>
      {[0, 1].map(i => renderDice(gameState.dice[i], gameState.players[i], gameState.current_player == i, gameState.captured[i]))}
      <button onClick={performAttack} disabled={gameState && gameState.players[gameState.current_player] !== playerId}>Attack</button>

      <AwaitingTurnGamesList gameId={gameId} />
    </div>
  );
};

const createGame = async () => {
  const gameId = await post(`/web4/contract/${contractId}/create_game`);

  console.log('Created game', gameId);
  // TODO: Push state to history instead?
  window.location.href = `/games/${gameId}`;
};

const App = () => {
  if (!playerId) {
    return (
      <div className="App">
        <h1>Login to play</h1>
        <a href="/web4/login">Login</a>
      </div>
    );
  }

  const path = window.location.pathname;
  const parts = path.split('/');

  if (path === '/') {
    return <>
      <button onClick={createGame}>Create Game</button>

      <OpenGamesList />
    </>
  }

  if (path.startsWith('/games/')) {
    const gameId = parts[2];
    return <Game gameId={gameId} />;
  }

  // Redirect to homepage for unknown paths
  window.location.href = '/';
};

export default App;
