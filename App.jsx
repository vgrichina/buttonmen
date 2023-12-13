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

const GameList = ({ games }) => (!games
  ? <div>Loading...</div>
  : <ul>
    {games.map(game => (
      <li key={game.id}>
        Game {game.id}: {game.players[0]} vs {game.players[1] || '???'} {
          game.players.some(p => p == playerId)
            ? <a href={`/games/${game.id}`}>Resume</a>
            : (game.players.some(p => p == "")
              ? <button onClick={() => joinGame(game.id)}>Join</button>
              : <a href={`/games/${game.id}`}>Spectate</a>)
        }
      </li>
    ))}
  </ul>
);

const usePolling = (deps, url, intervalMs = 2000) => {
  const [data, setData] = useState(null);

  useEffect(() => {
    const load = async () => {
      const data = await get(url);
      setData(data);
    }

    load();

    const interval = setInterval(load, intervalMs);
    return () => clearInterval(interval);
  }, [...deps, url, intervalMs]);

  return data;
};


const LatestGamesList = () => {
  const openGames = usePolling([], '/api/games');

  return (
    <div>
      <h2>Latest games created</h2>
      <GameList games={openGames} />
    </div>
  );
};

const AwaitingTurnGamesList = ({ gameId }) => {
  const games = usePolling([playerId], `/api/users/${playerId}/games`);
  const filteredGames = games?.filter(game => game.id !== gameId && game.current_player == game.players.indexOf(playerId));

  if (!filteredGames?.length) {
    return null;
  }

  return (
    <div>
      <h2>Awaiting your turn</h2>
      <GameList games={filteredGames} />
    </div>
  );
};

const Game = ({ gameId }) => {
  const [selectedDice, setSelectedDice] = useState([]); // To store indices of selected dice for an attack
  const [selectedDefenderDie, setSelectedDefenderDie] = useState(null); // Index of selected defender die

  const gameState = usePolling([gameId], `/api/games/${gameId}/status`);

  const attack = async (attackerDieIndices, defenderDieIndex) => {
    await post(`/web4/contract/${contractId}/attack`, { game_id: gameId, attacker_die_indices: attackerDieIndices, defender_die_index: defenderDieIndex });
  };

  const pass = async () => {
    await post(`/web4/contract/${contractId}/pass`, { game_id: gameId });
  }

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
    <div style={isActive ? { backgroundColor: 'rgb(255,247,230)' } : {}} >
      <h3>{dicePlayerId} {dicePlayerId == playerId && '(You)'}</h3>
      { isActive && dicePlayerId == playerId && <p><b>It's your turn</b></p> }
      <h4>Dice</h4>
      {playerDice.map((die, index) => {
        const isSelected = !isActive
          ? index === selectedDefenderDie
          : selectedDice.includes(index);
        return (
          <button
            key={index}
            onClick={() => !isActive ? selectDefenderDieForAttack(index) : selectDieForAttack(index)}
            style={{ backgroundColor: isSelected ? 'rgb(128,191,255)' : 'var(--button-base)' }}
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

  const currentPlayerIndex = gameState.players.indexOf(playerId);
  const otherPlayerIndex = (currentPlayerIndex + 1) % 2;

  return (
    <div>
      <h2>{gameState.players[0]} playing against {gameState.players[1]}</h2>
      <div class="this-player">
        {renderDice(gameState.dice[currentPlayerIndex], gameState.players[currentPlayerIndex], gameState.current_player === currentPlayerIndex, gameState.captured[currentPlayerIndex])}
      </div>
      <div class="other-player">
        {gameState.players[otherPlayerIndex] == '' ? <p><b>Waiting for player to join...</b></p>
          : renderDice(gameState.dice[otherPlayerIndex], gameState.players[otherPlayerIndex], gameState.current_player === otherPlayerIndex, gameState.captured[otherPlayerIndex])}
      </div>

      <button onClick={performAttack} disabled={gameState.players[gameState.current_player] !== playerId}>Attack</button>
      <button onClick={pass} disabled={gameState.players[gameState.current_player] !== playerId || !gameState.is_pass_allowed}>Pass</button>

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

const LoggedInBanner = () => (
  <p><a href="/">Home</a> | Logged in as {playerId} | <a href="/web4/logout">Logout</a></p>
);

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
      <LoggedInBanner />
      <button onClick={createGame}>Create Game</button>
      <LatestGamesList />

      <AwaitingTurnGamesList />
    </>
  }

  if (path.startsWith('/games/')) {
    const gameId = parts[2];
    return <>
      <LoggedInBanner />
      <Game gameId={gameId} />
    </>
  }

  // Redirect to homepage for unknown paths
  window.location.href = '/';
};

export default App;
